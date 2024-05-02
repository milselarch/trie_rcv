use std::cmp::{max, min};
use std::collections::{HashMap};
use petgraph::visit::Walker;
use std::borrow::BorrowMut;


use crate::vote;
use vote::VoteValues;
use crate::vote::{SpecialVotes, VoteStruct};

/*
edges from (level, choice) to (next_level, next_choice)
edge weight is number of votes with edge transition
*/

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

    fn search_or_create_child(&mut self, vote_value: VoteValues) -> &mut TrieNode {
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

struct RankedChoiceVoteTrie {
    root: TrieNode,
    dowdall_score_map: HashMap<u16, f32>
}

struct VoteTransferChanges<'a> {
    withhold_votes: u64, abstain_votes: u64,
    // (next candidate, next node, num votes to transfer to next candidate)
    vote_transfers: Vec<(u16, &'a TrieNode, u64)>
}

impl RankedChoiceVoteTrie {
    fn new() -> Self {
        RankedChoiceVoteTrie {
            root: TrieNode::new(),
            dowdall_score_map: Default::default()
        }
    }

    fn insert_vote<'a>(&mut self, vote: VoteStruct) {
        let mut current = &mut self.root;
        let vote_items = vote.iter().enumerate();

        for (ranking, vote_value) in vote_items {
            match vote_value {
                VoteValues::SpecialVote(_) => {}
                VoteValues::Candidate(candidate) => {
                    let score = *self.dowdall_score_map
                        .entry(candidate).or_insert(0f32);
                    let new_score = score + 1.0 / (ranking + 1) as f32;
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
                    transfer_changes.vote_transfers.push((
                        *next_candidate, next_node, next_node.num_votes
                    ));
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

    fn determine_winner<'a>(&self) -> Option<u16> {
        let mut candidate_vote_counts: HashMap<u16, u64> = HashMap::new();
        let mut frontier_nodes: HashMap<u16, Vec<&TrieNode>> = HashMap::new();
        let mut effective_total_votes: u64 = 0;
        let mut total_candidate_votes: u64 = 0;

        let kv_pairs_vec: Vec<(&VoteValues, &TrieNode)> =
            self.root.children.iter().collect();
        for (vote_value, node) in kv_pairs_vec {
            let candidate = match vote_value {
                VoteValues::SpecialVote(_) => { continue; }
                VoteValues::Candidate(candidate) => { candidate }
            };

            candidate_vote_counts.insert(*candidate, node.num_votes);
            frontier_nodes.insert(*candidate, vec![&node]);
            total_candidate_votes += node.num_votes;
            effective_total_votes += node.num_votes;
        }

        while candidate_vote_counts.len() > 0 {
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

            // find all candidates, nodes, and vote counts to transfer to
            let mut all_vote_transfers: Vec<VoteTransferChanges> = Vec::new();
            for weakest_candidate in weakest_candidates {
                let optional_weak_candidate_nodes =
                    frontier_nodes.get(&weakest_candidate);
                let candidate_nodes = match optional_weak_candidate_nodes {
                    None => { continue; }
                    Some(candidate_nodes) => { candidate_nodes }
                };

                for &node in candidate_nodes {
                    let transfer_result = self.transfer_next_votes(&node);
                    all_vote_transfers.push(transfer_result);
                }

                candidate_vote_counts.remove(&weakest_candidate);
                frontier_nodes.remove(&weakest_candidate);
            }

            // conduct vote transfers to next candidates and nodes
            for vote_transfer in all_vote_transfers {
                total_candidate_votes -=
                    vote_transfer.abstain_votes + vote_transfer.withhold_votes;
                effective_total_votes -= vote_transfer.abstain_votes;

                for vote_transfer in vote_transfer.vote_transfers {
                    let next_candidate = vote_transfer.0;
                    let next_node = vote_transfer.1;
                    let vote_allocation = vote_transfer.2;

                    let next_candidate_votes = candidate_vote_counts
                        .entry(next_candidate).or_insert(0);
                    let next_candidate_nodes = frontier_nodes
                        .entry(next_candidate).or_insert(Vec::new());

                    *next_candidate_votes += vote_allocation;
                    next_candidate_nodes.push(next_node);
                }
            }
        }

        return None;
    }
}
