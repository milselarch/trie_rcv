use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use petgraph::prelude::*;
use itertools::Itertools;

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
        return if let Some(node_ref) = self.children.get_mut(&vote_value) {
            node_ref
        } else {
            let node = TrieNode::new();
            let node_ref = self.children.entry(vote_value).or_insert(node);
            node_ref
        }
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

impl RankedChoiceVoteTrie {
    fn new() -> Self {
        RankedChoiceVoteTrie {
            root: TrieNode::new(),
            dowdall_score_map: Default::default()
        }
    }

    fn insert_vote(&mut self, vote: VoteStruct) {
        let mut current = &mut self.root;

        let _ = vote.iter().enumerate().inspect(|(ranking, vote_value)| {
            match vote_value {
                Some(VoteValues::Candidate(candidate)) => {
                    let score_option = self.dowdall_score_map.get(candidate);
                    let score = score_option.unwrap_or(&0f32);
                    let new_score = score + 1.0 / f32::from(*ranking + 1);
                    self.dowdall_score_map.insert(*candidate, new_score);
                }
            }
            let child = current.search_or_create_child(*vote_value);
            child.num_votes += 1;
            current = child;
        });
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

    fn transfer_next_votes(
        &self, node: TrieNode, frontier_nodes: &mut HashMap<u16, Vec<&TrieNode>>,
        effective_total_votes: &mut u64, total_candidate_votes: &mut u64,
        candidate_vote_counts: &mut HashMap<u16, u64>
    ) {
        let child_nodes = &node.children;

        for (next_vote_value, next_node) in child_nodes {
            // TODO: create key value pair if it doesnt exist
            match next_vote_value {
                VoteValues::SpecialVote(special_vote) => {
                    *effective_total_votes -= 1;

                    match special_vote {
                        SpecialVotes::WITHHOLD => {},
                        SpecialVotes::ABSTAIN => {
                            *total_candidate_votes -= 1;
                        }
                    }
                },
                VoteValues::Candidate(next_candidate) => {
                    let mut next_candidate_nodes =
                        frontier_nodes.get(&next_candidate).unwrap();
                    next_candidate_nodes.push(&next_node);

                    let mut next_candidate_vote_count =
                        candidate_vote_counts.get(&next_candidate)
                        .unwrap_or(&0);

                    next_candidate_vote_count += *next_node.num_votes;
                    candidate_vote_counts.insert(
                        *next_candidate, *next_candidate_vote_count
                    );
                }
            }
        }
    }

    fn determine_winner(&self) -> Option<u16> {
        let mut candidate_vote_counts: HashMap<u16, u64> = HashMap::new();
        let mut frontier_nodes: HashMap<u16, Vec<&TrieNode>> = HashMap::new();
        let mut effective_total_votes: u64 = 0;
        let mut total_candidate_votes: u64 = 0;

        for (vote_value, node) in self.root.children {
            match vote_value {
                VoteValues::SpecialVote(_) => { continue; }
                VoteValues::Candidate(candidate) => {
                    candidate_vote_counts.insert(candidate, node.num_votes);
                    frontier_nodes.insert(candidate, vec![&node]);
                    total_candidate_votes += node.num_votes;
                }
            }

            effective_total_votes += node.num_votes;
        }

        while candidate_vote_counts.len() > 0 {
            let mut min_candidate_votes: u64 = u64::MAX;
            if (total_candidate_votes <= effective_total_votes / 2) {
                return None;
            }

            for (candidate, num_votes) in &candidate_vote_counts {
                min_candidate_votes = min(min_candidate_votes, *num_votes);
                if (num_votes > *effective_total_votes / 2) {
                    return Some(*candidate)
                }
            }

            let mut weakest_candidates: Vec<u16> = Vec::new();
            for (candidate, num_votes) in &candidate_vote_counts {
                if *num_votes == min_candidate_votes {
                    weakest_candidates.push(*candidate);
                }
            }

            for weakest_candidate in weakest_candidates {
                let candidate_nodes =
                    frontier_nodes.get(&weakest_candidate).unwrap();

                for &&node in candidate_nodes {
                    self.transfer_next_votes(
                        node, &mut frontier_nodes, &mut effective_total_votes,
                        &mut total_candidate_votes, &mut candidate_vote_counts
                    );
                }
                candidate_vote_counts.remove(&weakest_candidate);
            }
        }

        return None;
    }
}
