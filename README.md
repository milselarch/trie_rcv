# trie_rcv
[https://crates.io/crates/trie_rcv](https://crates.io/crates/trie_rcv)  
Ranked Choice Voting (RCV) implementation using Tries in Rust.  

RCV differs from normal first past the post voting in that voters are allowed 
to rank candidates from most to least preferable. To determine the winner of an RCV election, the
least votes for the least popular candidate(s) are transferred to their next choice until 
some candidate reaches a majority.

Example usage:
```rust
use trie_rcv;
use trie_rcv::RankedChoiceVoteTrie;
use trie_rcv::vote::RankedVote;

fn main() {
    let mut rcv = RankedChoiceVoteTrie::new();
    rcv.set_elimination_strategy(EliminationStrategies::EliminateAll);

    rcv.insert_vote(RankedVote::from_vector(&vec![1, 2, 3, 4]).unwrap());
    rcv.insert_vote(RankedVote::from_vector(&vec![1, 2, 3]).unwrap());
    rcv.insert_vote(RankedVote::from_vector(&vec![3]).unwrap());
    rcv.insert_vote(RankedVote::from_vector(&vec![3, 2, 4]).unwrap());
    rcv.insert_vote(RankedVote::from_vector(&vec![4, 1]).unwrap());
    let winner = rcv.determine_winner();
    println!("WINNER = {:?}", winner);
    assert_eq!(winner, Some(1));
    
    // alternatively:
    let votes = RankedVote::from_vectors(&vec![
        vec![1, 2, 3, 4],
        vec![1, 2, 3],
        vec![3],
        vec![3, 2, 4],
        vec![4, 1]
    ]).unwrap();

    let winner2 = rcv.run_election(votes);
    println!("WINNER = {:?}", winner2);
    assert_eq!(winner2, Some(1));
}
```

This implementation also supports votes containing `withhold` and `abstain` votes, 
where the `withhold` vote allows the voter to declare for none of the candidates, and
the abstain vote allows the voter to voluntarily remove himself from the poll
(this is useful for improving the chances that the rest of the votes are able 
to conclude with a winning candidate)

```rust
use trie_rcv;
use trie_rcv::RankedChoiceVoteTrie;
use trie_rcv::vote::{RankedVote, SpecialVotes};

fn main() {
    let votes = RankedVote::from_vectors(&vec![
        vec![1, SpecialVotes::WITHHOLD.to_int()],
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
```

## Build instructions  
Build crate using `cargo build`, run integration tests with `cargo test`
