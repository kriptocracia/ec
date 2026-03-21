use std::path::Path;

use ec::counting::{Ballot, algorithm_for};
use ec::rules::load_rules;

#[test]
fn plurality_single_winner_simple_majority() {
    let rules = load_rules("plurality", Path::new("rules")).expect("load plurality rules");
    let algo = algorithm_for("plurality").expect("algorithm");

    // 5 ballots, candidate 1 has 3 votes, candidate 2 has 2 votes.
    let ballots: Vec<Ballot> = vec![vec![1], vec![1], vec![1], vec![2], vec![2]];

    let result = algo.count(&ballots, &rules).expect("count");

    assert_eq!(result.elected, vec![1]);
    let c1 = result.tally.iter().find(|t| t.candidate_id == 1).unwrap();
    let c2 = result.tally.iter().find(|t| t.candidate_id == 2).unwrap();
    assert_eq!(c1.votes, 3.0);
    assert_eq!(c2.votes, 2.0);
}

#[test]
fn plurality_tie_breaks_by_candidate_id() {
    let rules = load_rules("plurality", Path::new("rules")).expect("load plurality rules");
    let algo = algorithm_for("plurality").expect("algorithm");

    // 2 votes each: candidates 1 and 2. Tie; expect lowest id (1) elected.
    let ballots: Vec<Ballot> = vec![vec![1], vec![2], vec![1], vec![2]];

    let result = algo.count(&ballots, &rules).expect("count");
    assert_eq!(result.elected, vec![1]);
}

#[test]
fn plurality_multi_seat_top_two_elected() {
    let mut rules = load_rules("plurality", Path::new("rules")).expect("load plurality rules");
    // Override to a 2-seat election.
    rules.election.seats = 2;
    let algo = algorithm_for("plurality").expect("algorithm");

    // Candidate 1: 3 votes, candidate 2: 2 votes, candidate 3: 1 vote.
    let ballots: Vec<Ballot> = vec![vec![1], vec![1], vec![1], vec![2], vec![2], vec![3]];

    let result = algo.count(&ballots, &rules).expect("count");
    assert_eq!(result.elected, vec![1, 2]);
}
