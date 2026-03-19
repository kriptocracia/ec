use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rand::{rngs::StdRng, RngExt, SeedableRng};
use sha2::{Digest, Sha256};

use crate::counting::{
    Ballot, CandidateStatus, CandidateTally, CountResult, CountRound, CountingAlgorithm,
};
use crate::rules::{BallotMethod, ElectionRules};

/// STV implementation using Droop quota + Weighted Inclusive Gregory transfers.
/// Tie-break policy (temporary): deterministic by candidate_id.
pub struct StvAlgorithm;

#[derive(Clone)]
struct WeightedBallot {
    prefs: Vec<u8>,
    weight: f64,
    current: usize,
}

impl CountingAlgorithm for StvAlgorithm {
    fn count(&self, ballots: &[Ballot], rules: &ElectionRules) -> Result<CountResult> {
        if rules.ballot.method != BallotMethod::Ranked {
            anyhow::bail!("STV requires ranked ballots");
        }
        // Explicitly reject TOML options we do not support yet.
        if let Some(mode) = rules.counting.quota_mode.as_deref() {
            if mode != "static" {
                anyhow::bail!("NotImplemented: quota_mode={mode}");
            }
        }
        if let Some(criterion) = rules.counting.quota_criterion.as_deref() {
            if criterion != "gte" {
                anyhow::bail!("NotImplemented: quota_criterion={criterion}");
            }
        }
        if let Some(order) = rules.counting.surplus_order.as_deref() {
            if order != "by_size" {
                anyhow::bail!("NotImplemented: surplus_order={order}");
            }
        }
        if let Some(true) = rules.counting.bulk_exclusion {
            anyhow::bail!("NotImplemented: bulk_exclusion=true");
        }
        if let Some(false) = rules.counting.bulk_election {
            anyhow::bail!("NotImplemented: bulk_election=false");
        }

        let seats = rules.election.seats.max(1) as usize;

        // Build candidate set from ballots.
        let mut candidate_ids: BTreeSet<u8> = BTreeSet::new();
        for b in ballots {
            for &c in b {
                candidate_ids.insert(c);
            }
        }

        let mut status: BTreeMap<u8, CandidateStatus> = candidate_ids
            .iter()
            .map(|&id| (id, CandidateStatus::Active))
            .collect();

        let mut wb: Vec<WeightedBallot> = ballots
            .iter()
            .filter(|b| !b.is_empty())
            .map(|b| WeightedBallot {
                prefs: b.clone(),
                weight: 1.0,
                current: 0,
            })
            .collect();

        let total_valid: f64 = wb.len() as f64;
        if total_valid == 0.0 {
            return Ok(CountResult {
                elected: Vec::new(),
                tally: Vec::new(),
                count_sheet: Some(Vec::new()),
            });
        }

        // Droop quota: floor(V / (S + 1)) + 1
        let quota = (total_valid / ((seats as f64) + 1.0)).floor() + 1.0;

        let mut elected: Vec<u8> = Vec::new();
        let mut rounds: Vec<CountRound> = Vec::new();
        let mut round = 1_u32;
        let eps = 1e-9_f64;

        loop {
            // Tally is pure and does not mutate ballot assignment.
            let tallies_map = tally(&wb, &status, &candidate_ids);

            let mut tallies_vec: Vec<CandidateTally> = tallies_map
                .iter()
                .map(|(&id, &votes)| CandidateTally {
                    candidate_id: id,
                    votes,
                    status: *status.get(&id).unwrap_or(&CandidateStatus::Excluded),
                })
                .collect();

            tallies_vec.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));

            rounds.push(CountRound {
                round,
                tallies: tallies_vec,
                action: String::new(),
            });

            let active_left = status
                .values()
                .filter(|&&s| s == CandidateStatus::Active)
                .count();
            let seats_remaining = seats.saturating_sub(elected.len());
            if elected.len() >= seats || active_left == 0 {
                break;
            }
            if active_left <= seats_remaining {
                let active_ids: Vec<u8> = status
                    .iter()
                    .filter(|(_, s)| **s == CandidateStatus::Active)
                    .map(|(&id, _)| id)
                    .collect();
                for id in active_ids {
                    status.insert(id, CandidateStatus::Elected);
                    if !elected.contains(&id) {
                        elected.push(id);
                    }
                }
                break;
            }

            let tie_mode = rules
                .counting
                .tie_breaking
                .as_deref()
                .unwrap_or("deterministic-by-id");
            let tie_seed = rules
                .counting
                .tie_breaking_seed
                .unwrap_or_else(|| default_tie_seed(&candidate_ids));

            // Step B/C: elect one active candidate meeting quota (highest votes),
            // then transfer surplus using WIG.
            let mut winners: Vec<(u8, f64)> = tallies_map
                .iter()
                .filter(|(id, votes)| status[id] == CandidateStatus::Active && **votes + eps >= quota)
                .map(|(&id, &votes)| (id, votes))
                .collect();
            winners.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });

            if let Some((_, winner_votes)) = winners.first().copied() {
                let max_votes = winner_votes;
                let tied_winners: Vec<u8> = winners
                    .iter()
                    .filter(|(_, v)| approx_equal(*v, max_votes, eps))
                    .map(|(id, _)| *id)
                    .collect();
                let winner = resolve_tie(
                    &tied_winners,
                    &rounds,
                    tie_mode,
                    tie_seed,
                    TieObjective::HighestLosesFalse,
                )?;

                status.insert(winner, CandidateStatus::Elected);
                if !elected.contains(&winner) {
                    elected.push(winner);
                }

                let surplus = (winner_votes - quota).max(0.0);
                if surplus > eps && winner_votes > eps {
                    let transfer_value = surplus / winner_votes;
                    let mut transfer_ballots: Vec<WeightedBallot> = Vec::new();
                    for ballot in &mut wb {
                        if assigned_current(ballot) == Some(winner) {
                            let original_weight = ballot.weight;

                            // Retained part stays with elected candidate.
                            ballot.weight = original_weight * (1.0 - transfer_value);

                            // Transfer part moves forward to next active preference.
                            let mut tb = ballot.clone();
                            tb.weight = original_weight * transfer_value;
                            tb.current = ballot.current.saturating_add(1);
                            advance_to_next_active(&mut tb, &status);
                            if tb.current < tb.prefs.len() {
                                transfer_ballots.push(tb);
                            }
                        }
                    }
                    wb.extend(transfer_ballots);
                }

                if let Some(last) = rounds.last_mut() {
                    last.action = format!("Elected: {} (quota {:.4})", winner, quota);
                }
                round += 1;
                continue;
            }

            // Step D: no quota reached, exclude lowest active and transfer all its ballots at full weight.
            let mut active_tallies: Vec<(u8, f64)> = tallies_map
                .iter()
                .filter(|(id, _)| status[id] == CandidateStatus::Active)
                .map(|(&id, &v)| (id, v))
                .collect();

            if active_tallies.is_empty() {
                break;
            }

            active_tallies.sort_by(|a, b| {
                a.1.partial_cmp(&b.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });

            let min_votes = active_tallies[0].1;
            let tied_losers: Vec<u8> = active_tallies
                .iter()
                .filter(|(_, v)| approx_equal(*v, min_votes, eps))
                .map(|(id, _)| *id)
                .collect();
            let loser = resolve_tie(
                &tied_losers,
                &rounds,
                tie_mode,
                tie_seed,
                TieObjective::LowestLosesTrue,
            )?;
            let loser_votes = *tallies_map.get(&loser).unwrap_or(&0.0);
            status.insert(loser, CandidateStatus::Excluded);
            for ballot in &mut wb {
                if assigned_current(ballot) == Some(loser) {
                    ballot.current = ballot.current.saturating_add(1);
                    // Loser is already excluded, so advancing will skip it.
                    advance_to_next_active(ballot, &status);
                }
            }
            if let Some(last) = rounds.last_mut() {
                last.action = format!("Excluded: {} ({:.4})", loser, loser_votes);
            }

            round += 1;
        }

        // Final tally snapshot with final statuses.
        let final_tallies = tally(&wb, &status, &candidate_ids);
        let mut tally: Vec<CandidateTally> = final_tallies
            .iter()
            .map(|(&id, &votes)| CandidateTally {
                candidate_id: id,
                votes,
                status: *status.get(&id).unwrap_or(&CandidateStatus::Excluded),
            })
            .collect();
        tally.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));

        Ok(CountResult {
            elected,
            tally,
            count_sheet: Some(rounds),
        })
    }
}

fn tally(
    ballots: &[WeightedBallot],
    status: &BTreeMap<u8, CandidateStatus>,
    candidates: &BTreeSet<u8>,
) -> BTreeMap<u8, f64> {
    let mut tallies: BTreeMap<u8, f64> = candidates.iter().map(|&c| (c, 0.0)).collect();
    for ballot in ballots {
        if let Some(cid) = assigned_candidate(ballot, status) {
            *tallies.entry(cid).or_insert(0.0) += ballot.weight;
        }
    }
    tallies
}

fn next_active_pref_index(
    ballot: &WeightedBallot,
    status: &BTreeMap<u8, CandidateStatus>,
) -> Option<usize> {
    let mut i = ballot.current;
    while i < ballot.prefs.len() {
        let cid = ballot.prefs[i];
        if matches!(status.get(&cid), Some(CandidateStatus::Active)) {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn assigned_candidate(ballot: &WeightedBallot, status: &BTreeMap<u8, CandidateStatus>) -> Option<u8> {
    next_active_pref_index(ballot, status).map(|i| ballot.prefs[i])
}

fn assigned_current(ballot: &WeightedBallot) -> Option<u8> {
    ballot.prefs.get(ballot.current).copied()
}

fn advance_to_next_active(ballot: &mut WeightedBallot, status: &BTreeMap<u8, CandidateStatus>) {
    if let Some(i) = next_active_pref_index(ballot, status) {
        ballot.current = i;
    } else {
        ballot.current = ballot.prefs.len();
    }
}

#[derive(Clone, Copy)]
enum TieObjective {
    // Exclusion tie: lower historic value loses.
    LowestLosesTrue,
    // Winner tie: higher historic value wins.
    HighestLosesFalse,
}

fn resolve_tie(
    tied: &[u8],
    rounds: &[CountRound],
    mode: &str,
    seed: u64,
    objective: TieObjective,
) -> Result<u8> {
    if tied.is_empty() {
        anyhow::bail!("Cannot resolve tie for empty candidate set");
    }
    if tied.len() == 1 {
        return Ok(tied[0]);
    }

    match mode {
        "backwards" => Ok(break_tie_backwards(tied, rounds, objective)),
        "random" => Ok(break_tie_random(tied, seed)),
        "manual" => anyhow::bail!("NotImplemented: manual tie-break"),
        _ => Ok(*tied.iter().min().expect("non-empty tie set")),
    }
}

fn break_tie_backwards(tied: &[u8], rounds: &[CountRound], objective: TieObjective) -> u8 {
    // Skip current round and search backwards in history.
    for round in rounds.iter().rev().skip(1) {
        let mut votes: Vec<(u8, f64)> = tied
            .iter()
            .map(|id| {
                let v = round
                    .tallies
                    .iter()
                    .find(|t| t.candidate_id == *id)
                    .map(|t| t.votes)
                    .unwrap_or(0.0);
                (*id, v)
            })
            .collect();

        votes.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        let first = votes.first().copied();
        let last = votes.last().copied();
        if let (Some(f), Some(l)) = (first, last) {
            if !approx_equal(f.1, l.1, 1e-9) {
                return match objective {
                    TieObjective::LowestLosesTrue => f.0,
                    TieObjective::HighestLosesFalse => l.0,
                };
            }
        }
    }

    // Deterministic fallback.
    *tied.iter().min().expect("non-empty tie set")
}

fn break_tie_random(tied: &[u8], seed: u64) -> u8 {
    let mut rng = StdRng::seed_from_u64(seed);
    let idx = rng.random_range(0..tied.len());
    tied[idx]
}

fn default_tie_seed(candidate_ids: &BTreeSet<u8>) -> u64 {
    let mut hasher = Sha256::new();
    for id in candidate_ids {
        hasher.update([*id]);
    }
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(bytes)
}

fn approx_equal(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

