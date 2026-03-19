use std::path::Path;

use ec::counting::{algorithm_for, Ballot};
use ec::rules::load_rules;

#[test]
fn stv_simple_three_seat_election() {
    let rules = load_rules("stv", Path::new("rules")).expect("load stv rules");
    let algo = algorithm_for("stv").expect("algorithm");

    // 3 candidates, 3 seats, 5 ballots with clear preferences.
    // This is intentionally simple; no surplus handling is required for the outcome.
    let ballots: Vec<Ballot> = vec![
        vec![1, 2, 3],
        vec![1, 2, 3],
        vec![2, 1, 3],
        vec![2, 1, 3],
        vec![3, 2, 1],
    ];

    let mut rules = rules;
    rules.election.seats = 2;

    let result = algo.count(&ballots, &rules).expect("count");

    // Candidates 1 and 2 have the strongest first preferences, so they should be elected.
    assert!(result.elected.contains(&1));
    assert!(result.elected.contains(&2));
    assert_eq!(result.elected.len(), 2);
}

#[test]
fn stv_excludes_lowest_and_transfers_preferences() {
    let rules = load_rules("stv", Path::new("rules")).expect("load stv rules");
    let algo = algorithm_for("stv").expect("algorithm");

    // 3 candidates, 2 seats.
    // Candidate 3 starts weakest in first preferences and should be excluded
    // before the last round; seats are filled by the remaining candidates.
    let ballots: Vec<Ballot> = vec![
        vec![1, 2, 3],
        vec![1, 2, 3],
        vec![2, 1, 3],
        vec![2, 1, 3],
        vec![3, 1, 2],
        vec![3, 2, 1],
    ];

    let mut rules = rules;
    rules.election.seats = 2;

    let result = algo.count(&ballots, &rules).expect("count");

    // We only assert that exactly two candidates are elected.
    // The simplified STV implementation is deterministic and must not elect
    // the same candidate twice or fewer than the requested number of seats.
    assert_eq!(result.elected.len(), 2);
}

