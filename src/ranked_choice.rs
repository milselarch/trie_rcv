use std::collections::HashMap;
use petgraph::prelude::*;
use itertools::Itertools;

use crate::vote;
use vote::VoteValues;
use crate::vote::VoteStruct;

/*
edges from (level, choice) to (next_level, next_choice)
edge weight is number of votes with edge transition
*/

// ranking, vote choice value
type CandidateRanking = (u64, VoteValues);
// ranked-choice vote preference transition from one candidate to another
type CandidateTransition = (CandidateRanking, CandidateRanking);

struct RankedChoiceVoteGraph {
    graph: UnGraph<CandidateRanking, u64>,
    node_index_map: HashMap<CandidateRanking, NodeIndex>,
    edge_index_map: HashMap<CandidateTransition, EdgeIndex>
}

impl RankedChoiceVoteGraph {
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
        let mut ranking: u64 = 0;

        let callback = |
            &(prev_choice, next_choice): &(VoteValues, VoteValues)
        | {
            let prev_node: CandidateRanking = (ranking, prev_choice);
            let next_node: CandidateRanking = (ranking+1, next_choice);
            let transition: CandidateTransition = (prev_node, next_node);

            let prev_node_idx = self.get_node_idx(prev_node);
            let next_node_idx = self.get_node_idx(next_node);
            let edge_weight = self.get_edge_weight(transition);

            self.graph.update_edge(
                prev_node_idx, next_node_idx, edge_weight+1
            );
            ranking += 1;
        };

        let _ = vote.iter().tuple_windows().inspect(callback);
    }
}
