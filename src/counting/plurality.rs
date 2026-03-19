use std::collections::BTreeMap;

use anyhow::Result;

use crate::counting::{Ballot, CandidateStatus, CandidateTally, CountResult, CountingAlgorithm};
use crate::rules::ElectionRules;

/// Simple plurality (first-past-the-post) counting algorithm.
pub struct PluralityAlgorithm;

impl CountingAlgorithm for PluralityAlgorithm {
    fn count(&self, ballots: &[Ballot], rules: &ElectionRules) -> Result<CountResult> {
        let mut counts: BTreeMap<u8, f64> = BTreeMap::new();

        for ballot in ballots {
            if let Some(&candidate) = ballot.first() {
                *counts.entry(candidate).or_insert(0.0) += 1.0;
            }
        }

        // Determine number of seats.
        let seats = rules.election.seats.max(1) as usize;

        // Build tallies sorted by candidate_id ascending for determinism.
        let mut tallies: Vec<CandidateTally> = counts
            .iter()
            .map(|(&id, &votes)| CandidateTally {
                candidate_id: id,
                votes,
                status: CandidateStatus::Active,
            })
            .collect();

        // Sort by votes (desc), then candidate_id (asc).
        tallies.sort_by(|a, b| {
            b.votes
                .partial_cmp(&a.votes)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.candidate_id.cmp(&b.candidate_id))
        });

        // Mark elected candidates up to seats.
        let mut elected = Vec::new();
        for (idx, t) in tallies.iter_mut().enumerate() {
            if idx < seats {
                t.status = CandidateStatus::Elected;
                elected.push(t.candidate_id);
            } else {
                t.status = CandidateStatus::Excluded;
            }
        }

        Ok(CountResult {
            elected,
            tally: tallies,
            count_sheet: None,
        })
    }
}

