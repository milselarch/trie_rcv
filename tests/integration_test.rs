use trie_rcv;
use trie_rcv::RankedChoiceVoteTrie;
use trie_rcv::vote::VoteStruct;

#[test]
fn test_basic_scenario() {
      let votes = VoteStruct::from_vectors(vec![
        vec![1, 2, 3, 4],
        vec![1, 2, 3],
        vec![3],
        vec![3, 2, 4],
        vec![4, 1]
    ]).unwrap();

    println!("VOTES {}", votes.len());
    let rcv = RankedChoiceVoteTrie::new();
    let winner = rcv.run_election(votes);
    println!("WINNER {:?}", winner);
    assert_eq!(winner, Some(1));
}