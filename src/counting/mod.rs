use anyhow::Result;

use crate::rules::ElectionRules;

/// A single ballot as stored in SQLite: ordered list of candidate IDs.
/// Plurality: vec![3]
/// STV:       vec![3, 1, 4, 2]
pub type Ballot = Vec<u8>;

#[derive(Debug, Clone)]
pub struct CountResult {
    /// Elected candidate IDs, in order of election (for STV: order matters).
    pub elected: Vec<u8>,
    /// Full per-candidate vote totals or final transfer tallies.
    pub tally: Vec<CandidateTally>,
    /// Optional: serialized count sheet for STV (one entry per round).
    pub count_sheet: Option<Vec<CountRound>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateStatus {
    Active,
    Elected,
    Excluded,
}

impl CandidateStatus {
    /// Stable wire representation — independent of Rust variant names.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Elected => "elected",
            Self::Excluded => "excluded",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandidateTally {
    pub candidate_id: u8,
    pub votes: f64,
    pub status: CandidateStatus,
}

#[derive(Debug, Clone)]
pub struct CountRound {
    pub round: u32,
    pub tallies: Vec<CandidateTally>,
    pub action: String,
}

pub trait CountingAlgorithm: Send + Sync {
    fn count(&self, ballots: &[Ballot], rules: &ElectionRules) -> Result<CountResult>;
}

mod plurality;
mod stv;

use plurality::PluralityAlgorithm;
use stv::StvAlgorithm;

/// Registry: given a rules_id string, return the correct algorithm.
pub fn algorithm_for(rules_id: &str) -> Result<Box<dyn CountingAlgorithm>> {
    match rules_id {
        "plurality" => Ok(Box::new(PluralityAlgorithm)),
        "stv" => Ok(Box::new(StvAlgorithm)),
        other => anyhow::bail!("UNKNOWN_RULES: {}", other),
    }
}

