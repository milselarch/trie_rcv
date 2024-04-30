use std::cmp::max;
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

// ranking, vote choice value
type CandidateRanking = (u64, VoteValues);
// ranked-choice vote preference transition from one candidate to another
type CandidateTransition = (CandidateRanking, CandidateRanking);

struct RankedChoiceVoteGraph {
    graph: DiGraph<CandidateRanking, u64>,
    node_index_map: HashMap<CandidateRanking, NodeIndex>,
    edge_index_map: HashMap<CandidateTransition, EdgeIndex>,
    initial_votes: HashMap<VoteValues, u64>,
    score_map: HashMap<u64, f32>,
    max_vote_length: u64
}

impl RankedChoiceVoteGraph {
    fn init() -> RankedChoiceVoteGraph {
        return RankedChoiceVoteGraph {
            graph: Default::default(),
            node_index_map: Default::default(),
            edge_index_map: Default::default(),
            initial_votes: Default::default(),
            score_map: Default::default(),
            max_vote_length: 0,
        }
    }
    
    fn get_node_idx(&mut self, node: CandidateRanking) -> NodeIndex {
        let get_result = self.node_index_map.get(&node);
        return match get_result {
            Some(_node_idx) => { *_node_idx }
            None => {
                let node_idx = self.graph.add_node(node);
                self.node_index_map.insert(node, node_idx);
                return node_idx;
            }
        }
    }

    fn get_edge_idx(&mut self, transition: CandidateTransition) -> EdgeIndex {
        let prev_node_idx = self.get_node_idx(transition.0);
        let next_node_idx = self.get_node_idx(transition.1);
        let edge_find_result = self.graph.find_edge(
            prev_node_idx, next_node_idx
        );

        return if let Some(edge_index) = edge_find_result {
            edge_index
        } else {
            // Create edge if it doesn't exist
            let edge_index = self.graph.add_edge(
                prev_node_idx, next_node_idx, 0
            );
            self.edge_index_map.insert(transition, edge_index);
            return edge_index;
        }
    }

    fn get_edge_weight(&mut self, transition: CandidateTransition) -> u64 {
        let edge_index = self.get_edge_idx(transition);
        let edge_weight_result = self.graph.edge_weight_mut(edge_index);
        return if let Some(edge_weight) =
            edge_weight_result { *edge_weight } else { 0 }
    }

    fn add_vote(&mut self, vote: VoteStruct) {
        self.max_vote_length = max(self.max_vote_length, vote.len() as u64);

        let callback = |
            &(ranking, (prev_candidate, next_candidate)):
            &(usize, (VoteValues, VoteValues))
        | {
            if ranking == 0 {
                let first_choice = prev_candidate;
                let cache_get_result = self.initial_votes.get(&first_choice);

                if let Some(num_votes) = cache_get_result {
                    self.initial_votes.insert(first_choice, *num_votes+1);
                } else {
                    self.initial_votes.insert(first_choice, 1);
                }
            }

            let ranking_u64 = ranking as u64;
            let prev_node: CandidateRanking = (ranking_u64, prev_candidate);
            let next_node: CandidateRanking = (ranking_u64+1, next_candidate);

            let transition: CandidateTransition = (prev_node, next_node);
            let edge_index = self.get_edge_idx(transition);
            let edge_weight_result = self.graph.edge_weight_mut(edge_index);
            let mut edge_weight = edge_weight_result.unwrap();
            *edge_weight += 1;
        };

        let _ = vote.iter().tuple_windows().enumerate().inspect(callback);
    }

    fn determine_winner(&self) -> Option<u16> {
        let mut ranking_vote_counts: HashMap<VoteValues, u64>;
        ranking_vote_counts = HashMap::new();

        // let mut total_votes: u64 = 0;
        let mut total_effective_votes: u64 = 0;
        for (candidate, num_votes) in self.initial_votes.iter() {
            ranking_vote_counts.insert(*candidate, *num_votes);
            // total_votes += *num_votes;

            match candidate {
                VoteValues::SpecialVote(_) => { },
                VoteValues::Candidate(candidate) => {
                    total_effective_votes += *num_votes;
                },
            }
        }

        let mut next_ranking_vote_counts: HashMap<VoteValues, u64>;
        next_ranking_vote_counts = HashMap::new();

        for ranking in 0..self.max_vote_length {
            let mut min_candidate_votes = u64::MAX;

            // find the lowest number of votes a candidate
            // has on the current round
            for (preference, num_votes) in ranking_vote_counts.iter() {
                // candidate has reached a majority
                if (*num_votes > total_effective_votes / 2) {
                    return match *preference {
                        VoteValues::SpecialVote(_) => { None }
                        VoteValues::Candidate(candidate) => {
                            Some(candidate)
                        },
                    }
                }
                match *preference {
                    VoteValues::SpecialVote(_) => { continue }
                    VoteValues::Candidate(candidate) => {
                        min_candidate_votes = u64::min(
                            min_candidate_votes, *num_votes
                        );
                    },
                }
            }

            // find candidates who should be eliminated from current round
            let mut weakest_candidates: Vec<u16> = Vec::new();
            for (preference, num_votes) in ranking_vote_counts.iter() {
                match *preference {
                    VoteValues::SpecialVote(_) => { continue },
                    VoteValues::Candidate(candidate) => {
                        if (*num_votes == min_candidate_votes) {
                            weakest_candidates.push(candidate);
                        }
                    }
                }
            }

            for weakest_candidate in weakest_candidates {
                let node: CandidateRanking = (ranking, *weakest_candidate);
                let get_result = self.node_index_map.get(&node);
                let mut node_idx: NodeIndex;

                match get_result {
                    Some(_node_idx) => { node_idx = *_node_idx; },
                    _ => { continue }
                }

                let edges = self.graph.edges(node_idx).collect();
                for edge_idx in edges {
                    let edge = self.graph.edge_endpoints(edge_idx).unwrap();
                    let edge_weight = self.graph.edge_weight(edge_idx);
                    let next_node_idx = edge.1;
                    let next_node = self.graph[next_node_idx];
                }
            }

            if next_ranking_vote_counts.len() == 0 {
                return None;
            }
        }

        return None;
    }
}