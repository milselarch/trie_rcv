use itertools::all;
use trie_rcv;
use trie_rcv::{EliminationStrategies, RankedChoiceVoteTrie};
use trie_rcv::vote::{SpecialVotes, RankedVote};

const WITHOLD_VOTE_VAL: i32 = SpecialVotes::WITHHOLD.to_int();
const ABSTAIN_VOTE_VAL: i32 = SpecialVotes::ABSTAIN.to_int();

#[test]
fn test_basic_scenario() {
    let votes = RankedVote::from_vectors(&vec![
        vec![1, 2, 3, 4],
        vec![1, 2, 3],
        vec![3],
        vec![3, 2, 4],
        vec![4, 1]
    ]).unwrap();

    let rcv = RankedChoiceVoteTrie::new();
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(
        winner, Some(1),
        "Vote 4 > 1 should go to 1, leading to Candidate 1 winning"
    );
}

#[test]
fn test_vote_insert() {
    let mut rcv = RankedChoiceVoteTrie::new();
    rcv.set_elimination_strategy(EliminationStrategies::EliminateAll);

    rcv.insert_vote(RankedVote::from_vector(&vec![1, 2, 3, 4]).unwrap());
    rcv.insert_vote(RankedVote::from_vector(&vec![1, 2, 3]).unwrap());
    rcv.insert_vote(RankedVote::from_vector(&vec![3]).unwrap());
    rcv.insert_vote(RankedVote::from_vector(&vec![3, 2, 4]).unwrap());
    rcv.insert_vote(RankedVote::from_vector(&vec![4, 1]).unwrap());
    let winner = rcv.determine_winner();
    println!("WINNER = {:?}", winner);
    assert_eq!(
        winner, Some(1),
        "Vote 4 > 1 should go to 1, leading to Candidate 1 winning"
    );
}

#[test]
fn test_simple_majority() {
    let votes = RankedVote::from_vectors(&vec![
        vec![1, 2, 3, 4],
        vec![1, 2, 3],
        vec![3],
        vec![3, 2, 4],
        vec![1, 2]
    ]).unwrap();

    let rcv = RankedChoiceVoteTrie::new();
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(winner, Some(1), "Candidate 1 has majority");
}

#[test]
fn test_tie_scenario() {
    let votes = RankedVote::from_vectors(&vec![
        vec![1, 2],
        vec![2, 1]
    ]).unwrap();

    let rcv = RankedChoiceVoteTrie::new();
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(winner, None, "There should be a tie");
}

#[test]
fn test_withold_vote_end() {
    let votes = RankedVote::from_vectors(&vec![
        vec![1, WITHOLD_VOTE_VAL],
        vec![2, 1],
        vec![3, 2],
        vec![3]
    ]).unwrap();

    let rcv = RankedChoiceVoteTrie::new();
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(
        winner, None, concat![
        "Candidate 1's vote should not count after round 1, ",
        "no one should have majority"
    ]);
}

#[test]
fn test_abstain_vote_end() {
    let votes = RankedVote::from_vectors(&vec![
        vec![1, ABSTAIN_VOTE_VAL],
        vec![2, 1],
        vec![3, 2],
        vec![3]
    ]).unwrap();

    let rcv = RankedChoiceVoteTrie::new();
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(
        winner, Some(3), concat![
        "First vote is ignored in round 2, candidate 3 wins"
    ]);
}

#[test]
fn test_withhold_votes_only() {
    let votes = RankedVote::from_vectors(&vec![
        vec![WITHOLD_VOTE_VAL],
        vec![WITHOLD_VOTE_VAL],
        vec![WITHOLD_VOTE_VAL],
        vec![ABSTAIN_VOTE_VAL]
    ]).unwrap();

    let rcv = RankedChoiceVoteTrie::new();
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(winner, None);
}

#[test]
fn test_dowdall_elimination() {
    let votes = RankedVote::from_vectors(&vec![
        vec![1, 6, 15],
        vec![1, 2, 6, 15, 5, 4, 7, 3, 11],
        vec![6, 15, 1, 11, 10, 16, 17, 8, 2, 3, 5, 7],
        vec![9, 8, 6, 11, 13, 3, 1],
        vec![13, 14, 16, 6, 3, 4, 5, 2, 1, 8, 9]
    ]).unwrap();

    let rcv = RankedChoiceVoteTrie::new();
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(winner, Some(6));
}

#[test]
fn test_all_elimination() {
    let votes = RankedVote::from_vectors(&vec![
        vec![1, 6, 15],
        vec![1, 2, 6, 15, 5, 4, 7, 3, 11],
        vec![6, 15, 1, 11, 10, 16, 17, 8, 2, 3, 5, 7],
        vec![9, 8, 6, 11, 13, 3, 1],
        vec![13, 14, 16, 6, 3, 4, 5, 2, 1, 8, 9]
    ]).unwrap();

    let mut rcv = RankedChoiceVoteTrie::new();
    rcv.set_elimination_strategy(EliminationStrategies::EliminateAll);
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(winner, Some(1));
}

#[test]
fn test_spoiler_vote() {
    const T: i32 = 3;
    const S: i32 = 2;
    const B: i32 = 1;

    let rcv_vote_type1 = vec![vec![S, B, T]];
    let rcv_vote_type2 = vec![vec![B, S, T]];
    let rcv_vote_type3 = vec![vec![B, T, S]];
    let rcv_vote_type4 = vec![vec![T, B, S]];

    fn repeat(num_votes: u64, vote_type: Vec<Vec<i32>>) -> Vec<Vec<i32>> {
        return (0..num_votes)
        .flat_map(|_| vote_type.clone())
        .collect::<Vec<_>>();
    }

    let mut raw_votes: Vec<Vec<i32>> = vec![];
    raw_votes.extend(repeat(35, rcv_vote_type1));
    raw_votes.extend(repeat(10, rcv_vote_type2));
    raw_votes.extend(repeat(10, rcv_vote_type3));
    raw_votes.extend(repeat(45, rcv_vote_type4));

    let votes = RankedVote::from_vectors(&raw_votes).unwrap();
    let mut rcv = RankedChoiceVoteTrie::new();
    rcv.set_elimination_strategy(EliminationStrategies::RankedPairs);
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(winner, Some(T as u16));
}

#[test]
fn test_condorcet_vote() {
    const T: i32 = 3;
    const S: i32 = 2;
    const B: i32 = 1;

    let rcv_vote_type1 = vec![vec![S, B, T]];
    let rcv_vote_type2 = vec![vec![B, S, T]];
    let rcv_vote_type3 = vec![vec![B, T, S]];
    let rcv_vote_type4 = vec![vec![T, B, S]];

    fn repeat(num_votes: u64, vote_type: Vec<Vec<i32>>) -> Vec<Vec<i32>> {
        return (0..num_votes)
        .flat_map(|_| vote_type.clone())
        .collect::<Vec<_>>();
    }

    let mut raw_votes: Vec<Vec<i32>> = vec![];
    raw_votes.extend(repeat(35, rcv_vote_type1));
    raw_votes.extend(repeat(10, rcv_vote_type2));
    raw_votes.extend(repeat(10, rcv_vote_type3));
    raw_votes.extend(repeat(45, rcv_vote_type4));

    let votes = RankedVote::from_vectors(&raw_votes).unwrap();
    let mut rcv = RankedChoiceVoteTrie::new();
    rcv.set_elimination_strategy(EliminationStrategies::CondorcetRankedPairs);
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(winner, Some(B as u16));
}