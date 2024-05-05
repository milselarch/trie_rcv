# trie_rcv
[https://crates.io/crates/trie_rcv](https://crates.io/crates/trie_rcv)  
Ranked Choice Voting (RCV) implementation using Tries in Rust.  

RCV differs from normal first past the post voting in that voters are allowed 
to rank candidates from most to least preferable. To determine the winner of an RCV election, the
votes for the least popular candidate each round is transferred to the next candidate in the
respective ranked votes until some candidate achieves an effective majority.

Example usage:
```rust
use trie_rcv;
use trie_rcv::RankedChoiceVoteTrie;
use trie_rcv::vote::RankedVote;

fn main() {
    let mut rcv = RankedChoiceVoteTrie::new();
    // remove all candidates with the lowest number of votes each round
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

This implementation also supports ranked votes ending 
with `SpecialVotes::WITHHOLD` and `SpecialVotes::ABSTAIN` values:
1. `SpecialVotes::WITHHOLD`   
Allows the voter to explicitly declare for none of the candidates.  
Qualitatively this allows a voter to declare a vote of no confidence.  
Serializes to `-1` via `SpecialVotes::WITHHOLD.to_int()`
2. `SpecialVotes::ABSTAIN`  
Allows the voter to explicitly declare for none of the candidates while also 
voluntarily removing himself from the poll.   
Qualitatively this allows a voter to indicate
that he wants one of the candidates to win but isn't able to decide for himself and 
would thus want to delegate the decision to the rest of the electorate.  
Serializes to `-2` via `SpecialVotes::ABSTAIN.to_int()`

```rust
use trie_rcv;
use trie_rcv::RankedChoiceVoteTrie;
use trie_rcv::vote::{RankedVote, SpecialVotes};

fn main() {
    let rcv = RankedChoiceVoteTrie::new();

    let votes_round1 = RankedVote::from_vectors(&vec![
        vec![1, SpecialVotes::WITHHOLD.to_int()],
        vec![2, 1],
        vec![3, 2],
        vec![3]
    ]).unwrap();

    let winner_round1 = rcv.run_election(votes_round1);
    println!("WINNER = {:?}", winner);
    assert_eq!(
        winner_round1, None, concat![
        "Candidate 1's vote should not count after round 1, ",
        "no one should have majority"
    ]);
    
    let votes_round2 = RankedVote::from_vectors(&vec![
        vec![1, SpecialVotes::ABSTAIN.to_int()],
        vec![2, 1],
        vec![3, 2],
        vec![3]
    ]).unwrap();

    let winner_round2 = rcv.run_election(votes_round2);
    println!("WINNER = {:?}", winner_round2);
    assert_eq!(
        winner_round2, Some(3), concat![
       "First vote is ignored in round 2, candidate 3 wins"
    ]);
}
```

### Elimination Strategies
Technically the RCV algorithm specification doesn't state what to do in the situation that
there are multiple candidates who all have the same, lowest number of votes in some round during
RCV. The `elimination_strategy` setting handles how to deal with this edge case:

1. `EliminationStrategies::ElimnateAll`  
Removes all candidates with the lowest number of votes each round.
2. `EliminationStrategies::DowdallScoring` (default)  
Among multiple candidates with the lowest number of votes each round,
sort the candidates by their dowdall score and eliminate the candidate(s)
with the lowest [Dowdall score](https://rdrr.io/cran/votesys/man/dowdall_method.html). 
The Dowdall score for each candidate is calculated by
the sum of the  inverse of the ranking (starting from 1) for each ranked vote. 
If a ranked vote does not contain a candidate, then it does not count 
towards the dowdall score)
3. `EliminationStrategies::RankedPairs`  
Among multiple candidates with the lowest number of votes each round, attempt
to construct  a directed acyclic graph establishing a pecking order between
these candidates via [ranked-pair](https://en.wikipedia.org/wiki/Ranked_pairs) 
comparisons, whereby candidate A is said to be better than candidate B 
if there are more votes that rank A higher than B
than vice versa.
   1. Note that if a ranked vote ranks A explicitly but not B, then that is
   counted as ranking A higher than B as well
   2. In the event that a directed acyclic preference graph cannot be established
   (such as when there are cyclic preferences between candidates), then the elimination
   behavior will default to eliminating all candidates with the same, 
   lowest number of votes each round i.e. it will fall back to the 
   behavior of `EliminationStrategies::ElimnateAll` 
   
## Build instructions  
Build crate using `cargo build`, run integration tests with `cargo test`
