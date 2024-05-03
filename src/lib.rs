use std::cmp::min;
use std::collections::HashMap;
pub use vote::*;

pub mod vote;

#[derive(Default)]
struct TrieNode {
    children: HashMap<VoteValues, TrieNode>,
    num_votes: u64
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            num_votes: 0,
        }
    }

    fn search_or_create_child(
        &mut self, vote_value: VoteValues
    ) -> &mut TrieNode {
        self.children.entry(vote_value).or_insert(TrieNode::new())
    }

    fn search_child(&self, vote_value: VoteValues) -> Option<&TrieNode> {
        return if let Some(node_ref) = self.children.get(&vote_value) {
            Some(node_ref)
        } else {
            None
        }
    }
}

pub struct RankedChoiceVoteTrie {
    root: TrieNode,
    dowdall_score_map: HashMap<u16, f32>,
    elimination_strategy: EliminationStrategies
}

struct VoteTransfer<'a> {
    next_candidate: u16,
    next_node: Box<&'a TrieNode>,
    num_votes: u64
}

struct VoteTransferChanges<'a> {
    withhold_votes: u64, abstain_votes: u64,
    // (next candidate, next node, num votes to transfer to next candidate)
    vote_transfers: Vec<VoteTransfer<'a>>
}

#[derive(Clone)]
pub enum EliminationStrategies {
    EliminateAll, DowdallScoring
}

impl RankedChoiceVoteTrie {
    pub fn new() -> Self {
        RankedChoiceVoteTrie {
            root: TrieNode::new(),
            dowdall_score_map: Default::default(),
            elimination_strategy: EliminationStrategies::DowdallScoring,
        }
    }

    pub fn set_elimination_strategy(&mut self, strategy: EliminationStrategies) {
        self.elimination_strategy = strategy;
    }

    pub fn insert_votes(&mut self, votes: Vec<VoteStruct>) {
        for vote in votes {
            self.insert_vote(vote);
        }
    }

    pub fn insert_vote<'a>(&mut self, vote: VoteStruct) {
        let mut current = &mut self.root;
        let vote_items = vote.iter().enumerate();

        for (ranking, vote_value) in vote_items {
            // println!("ITEM {}", ranking);
            match vote_value {
                VoteValues::SpecialVote(_) => {}
                VoteValues::Candidate(candidate) => {
                    let score = *self.dowdall_score_map
                        .entry(candidate).or_insert(0f32);
                    let new_score = score + 1.0 / (ranking + 1) as f32;
                    assert!(new_score.is_finite());
                    self.dowdall_score_map.insert(candidate, new_score);
                }
            }
            let child = current.search_or_create_child(vote_value);
            child.num_votes += 1;
            current = child;
        };
    }

    fn search_node(&mut self, votes: Vec<VoteValues>) -> Option<&mut TrieNode> {
        let mut current = &mut self.root;
        for vote_value in votes {
            if let Some(node) = current.children.get_mut(&vote_value) {
                current = node;
            } else {
                return None;
            }
        }
        return Some(current);
    }

    fn transfer_next_votes<'a>(&'a self, node: &'a TrieNode) -> VoteTransferChanges {
        let child_nodes = &node.children;
        let mut transfer_changes = VoteTransferChanges {
            withhold_votes: 0, abstain_votes: 0,
            vote_transfers: Default::default(),
        };

        for (next_vote_value, next_node) in child_nodes {
            match next_vote_value {
                VoteValues::SpecialVote(special_vote) => {
                    match special_vote {
                        SpecialVotes::WITHHOLD => {
                            transfer_changes.withhold_votes += 1;
                        },
                        SpecialVotes::ABSTAIN => {
                            transfer_changes.abstain_votes += 1;
                        }
                    }
                },
                VoteValues::Candidate(next_candidate) => {
                    transfer_changes.vote_transfers.push(VoteTransfer{
                        next_candidate: *next_candidate,
                        next_node: Box::new(next_node),
                        num_votes: next_node.num_votes
                    });
                }
            }
        }

        return transfer_changes;
    }

    fn get_or_create_nodes<'a>(
        &'a self, candidate: &u16,
        frontier_nodes: &'a mut HashMap<u16, Vec<&'a TrieNode>>
    ) -> &mut Vec<&TrieNode> {
        frontier_nodes.entry(*candidate).or_insert(Vec::new())
    }

    fn find_dowdall_weakest(&self, candidates: Vec<u16>) -> Vec<u16> {
        /*
        returns the subset of candidates from the input candidates vector
        that score the lowest according the dowdall scoring criteria
        */
        let mut min_score = f32::MAX;
        let mut weakest_candidates: Vec<u16> = Vec::new();

        for candidate in &candidates {
            let score = self.dowdall_score_map.get(candidate)
                .expect("score map should have scores for all candidates");
            min_score = f32::min(*score, min_score);
        }

        for candidate in &candidates {
            let score = self.dowdall_score_map.get(candidate)
                .expect("score map should have scores for all candidates");
            if f32::eq(score, &min_score) {
                weakest_candidates.push(*candidate);
            }
        }

        return weakest_candidates;
    }

    pub fn run_election(&self, votes: Vec<VoteStruct>) -> Option<u16> {
        let mut rcv = RankedChoiceVoteTrie {
            root: Default::default(),
            dowdall_score_map: Default::default(),
            elimination_strategy: self.elimination_strategy.clone()
        };
        rcv.insert_votes(votes);
        return rcv.determine_winner();
    }

    pub fn determine_winner<'a>(&self) -> Option<u16> {
        // println!("RUN_ELECTION_START");
        let mut candidate_vote_counts: HashMap<u16, u64> = HashMap::new();
        // trie frontier nodes are stored in boxes so that they will be
        // allocated on the heap, otherwise they might cause a stackoverflow
        let mut frontier_nodes:
            HashMap<u16, Vec<Box<&TrieNode>>> = HashMap::new();
        // total number of voters (who have no abstained from vote)
        let mut effective_total_votes: u64 = 0;
        // total number of votes that go to candidates
        let mut total_candidate_votes: u64 = 0;

        let kv_pairs_vec: Vec<(&VoteValues, &TrieNode)> =
            self.root.children.iter().collect();
        for (vote_value, node) in kv_pairs_vec {
            match vote_value {
                VoteValues::SpecialVote(SpecialVotes::ABSTAIN) => {}
                VoteValues::SpecialVote(SpecialVotes::WITHHOLD) => {
                    effective_total_votes += node.num_votes;
                }
                VoteValues::Candidate(candidate) => {
                    candidate_vote_counts.insert(*candidate, node.num_votes);
                    frontier_nodes.insert(*candidate, vec![Box::new(node)]);
                    total_candidate_votes += node.num_votes;
                    effective_total_votes += node.num_votes;
                }
            };
        }

        while candidate_vote_counts.len() > 0 {
            // println!("COUNTS {:?}", candidate_vote_counts);
            let mut min_candidate_votes: u64 = u64::MAX;
            // impossible for any candidate to win as sum of
            // candidate votes is under the total number of votes casted
            if total_candidate_votes <= effective_total_votes / 2 {
                return None;
            }

            for (candidate, num_votes) in &candidate_vote_counts {
                min_candidate_votes = min(min_candidate_votes, *num_votes);
                // some candidate has won a majority of the votes
                if *num_votes > effective_total_votes / 2 {
                    return Some(*candidate)
                }
            }

            // find candidates with the lowest number of effective votes
            let mut weakest_candidates: Vec<u16> = Vec::new();
            for (candidate, num_votes) in &candidate_vote_counts {
                if *num_votes == min_candidate_votes {
                    weakest_candidates.push(*candidate);
                }
            }

            // further filter down candidates to eliminate using
            // specified elimination strategy
            match self.elimination_strategy {
                EliminationStrategies::EliminateAll => {},
                EliminationStrategies::DowdallScoring => {
                    weakest_candidates = self.find_dowdall_weakest(weakest_candidates);
                }
            }

            // find all candidates, nodes, and vote counts to transfer to
            let mut all_vote_transfers: Vec<VoteTransfer> = Vec::new();
            let mut new_withhold_votes: u64 = 0;
            let mut new_abstain_votes: u64 = 0;

            for weakest_candidate in weakest_candidates {
                let candidate_nodes = frontier_nodes.get(&weakest_candidate)
                    .expect("all uneliminated candidates must have node(s)");

                for box_node in candidate_nodes {
                    let transfer_result = self.transfer_next_votes(box_node);
                    new_abstain_votes += transfer_result.abstain_votes;
                    new_withhold_votes += transfer_result.withhold_votes;
                    all_vote_transfers.extend(transfer_result.vote_transfers);
                }

                candidate_vote_counts.remove(&weakest_candidate);
                frontier_nodes.remove(&weakest_candidate);
            }

            // 0 vote transfers will be done, election is unable to progress
            if all_vote_transfers.len() == 0 { return None; }
            total_candidate_votes -= new_abstain_votes + new_withhold_votes;
            effective_total_votes -= new_abstain_votes;

            // conduct vote transfers to next candidates and trie nodes
            for vote_transfer in all_vote_transfers {
                let next_candidate = vote_transfer.next_candidate;
                let vote_allocation = vote_transfer.num_votes;
                assert!(vote_allocation > 0);

                let next_candidate_votes = candidate_vote_counts
                    .entry(next_candidate).or_insert(0);
                let next_candidate_nodes = frontier_nodes
                    .entry(next_candidate).or_insert(Vec::new());

                *next_candidate_votes += vote_allocation;
                next_candidate_nodes.push(vote_transfer.next_node);
            }
        }

        return None;
    }
}
