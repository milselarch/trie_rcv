use trie_rcv;
use trie_rcv::RankedChoiceVoteTrie;
use trie_rcv::vote::{SpecialVotes, VoteStruct};

const WITHOLD_VOTE_VAL: i32 = SpecialVotes::WITHHOLD.to_int();
const ABSTAIN_VOTE_VAL: i32 = SpecialVotes::ABSTAIN.to_int();

#[test]
fn test_basic_scenario() {
    let votes = VoteStruct::from_vectors(&vec![
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
fn test_simple_majority() {
    let votes = VoteStruct::from_vectors(&vec![
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
    let votes = VoteStruct::from_vectors(&vec![
        vec![1, 2],
        vec![2, 1]
    ]).unwrap();

    let rcv = RankedChoiceVoteTrie::new();
    let winner = rcv.run_election(votes);
    println!("WINNER = {:?}", winner);
    assert_eq!(winner, None, "There should be a tie");
}

#[test]
fn test_zero_vote_end() {
    let votes = VoteStruct::from_vectors(&vec![
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