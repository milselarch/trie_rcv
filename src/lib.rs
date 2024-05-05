use std::cmp::min;
use std::collections::{HashMap, HashSet};
use petgraph::graph::{DiGraph, NodeIndex};
use itertools::{iproduct, Itertools};
use std::collections::VecDeque;
use petgraph::Direction;
use petgraph::prelude::EdgeRef;

pub use vote::*;

pub mod vote;

#[derive(PartialEq)]
pub enum PairPreferences {
    PreferredOver, Inconclusive, PreferredAgainst
}

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
    elimination_strategy: EliminationStrategies,
    unique_candidates: HashSet<u16>
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

// strategies for how to eliminate candidates each round
#[derive(Clone, PartialEq)]
pub enum EliminationStrategies {
    // removes all candidates with the lowest number of votes each round
    EliminateAll,
    // eliminate the candidate(s) with both the lowest number of votes
    // followed by the lowest dowdall score
    DowdallScoring,
    // eliminate the candidate(s) with both the lowest number of votes
    // and who lose against other candidates with the same number of votes
    // in a head-to-head comparison
    RankedPairs
}

fn is_graph_acyclic(graph: &DiGraph<u16, u64>) -> bool {
    /*
    checks if there doesn't exist any path of directed edges
    from some edge in the graph back to itself
    */
    if graph.node_count() == 0 { return true }
    let nodes: Vec<NodeIndex> = graph.node_indices().collect();
    let mut all_explored_nodes = HashSet::<NodeIndex>::new();

    fn dfs_find_cycle(
        node: &NodeIndex, path: &mut Vec<NodeIndex>,
        explored: &mut HashSet::<NodeIndex>, graph: &DiGraph<u16, u64>
    ) -> bool {
        // use DFS to see if a cycle can be created from paths starting from node
        explored.insert(*node);

        // get neighbors of node where there is an
        // outgoing edge from node to neighbor
        let directed_neighbors: Vec<NodeIndex> = graph
            .edges_directed(*node, petgraph::Direction::Outgoing)
            .map(|edge| { edge.target()} )
            .collect();

        for neighbor in directed_neighbors {
            if path.contains(&neighbor) { return true }
            path.push(neighbor);
            let has_cycle = dfs_find_cycle(&neighbor, path, explored, graph);
            path.pop();

            if has_cycle { return true }
        }

        return false;
    }

    for node in nodes {
        if all_explored_nodes.contains(&node) { continue }
        let mut dfs_explored_nodes = HashSet::<NodeIndex>::new();
        let has_cycle = dfs_find_cycle(
            &node, &mut Vec::<NodeIndex>::new(), &mut dfs_explored_nodes, graph
        );

        if has_cycle { return false }
        all_explored_nodes.extend(dfs_explored_nodes.iter().collect_vec());
    }

    return true;
}

fn is_graph_weakly_connected(graph: &DiGraph<u16, u64>) -> bool {
    /*
    checks if there is a path from every node to every other
    node when all the edges are converted from directed to undirected
    */
    if graph.node_count() == 0 { return true }
    let mut queue = VecDeque::<NodeIndex>::new();
    let mut explored_nodes = HashSet::<NodeIndex>::new();
    let nodes: Vec<NodeIndex> = graph.node_indices().collect();
    let start_node = nodes[0];
    queue.push_back(start_node);

    let get_undirected_neighbors = |node: NodeIndex| {
        let mut neighbors = Vec::<NodeIndex>::new();
        neighbors.extend(graph.neighbors_directed(node, Direction::Incoming));
        neighbors.extend(graph.neighbors_directed(node, Direction::Outgoing));
        return neighbors;
    };

    // do a DFS search to see if all nodes are reachable from start_node
    loop {
        let node = match queue.pop_back() {
            None => { break; }
            Some(node) => node
        };

        if explored_nodes.contains(&node) { continue }
        explored_nodes.insert(node);

        let neighbors: Vec<NodeIndex> = get_undirected_neighbors(node);
        // println!("DFS {:?}", (node, &neighbors));
        for neighbor in neighbors {
            queue.push_back(neighbor)
        }
    }

    return explored_nodes.len() == graph.node_count()
}

impl RankedChoiceVoteTrie {
    pub fn new() -> Self {
        RankedChoiceVoteTrie {
            root: TrieNode::new(),
            dowdall_score_map: Default::default(),
            elimination_strategy: EliminationStrategies::DowdallScoring,
            unique_candidates: Default::default(),
        }
    }

    pub fn set_elimination_strategy(&mut self, strategy: EliminationStrategies) {
        self.elimination_strategy = strategy;
    }

    pub fn insert_votes(&mut self, votes: Vec<RankedVote>) {
        for vote in votes {
            self.insert_vote(vote);
        }
    }

    pub fn insert_vote<'a>(&mut self, vote: RankedVote) {
        let mut current = &mut self.root;
        let vote_items = vote.iter().enumerate();
        current.num_votes += 1;

        for (ranking, vote_value) in vote_items {
            // println!("ITEM {}", ranking);
            match vote_value {
                VoteValues::SpecialVote(_) => {}
                VoteValues::Candidate(candidate) => {
                    self.unique_candidates.insert(candidate);
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

    fn find_ranked_pairs_weakest(
        &self, candidates: Vec<u16>,
        ranked_pairs_map: &HashMap<(u16, u16), u64>
    ) -> Vec<u16> {
        // println!("\n----------------");
        // println!("PRE_RANK_FILTER {:?}", candidates);
        // println!("PAIRS_MAP {:?}", ranked_pairs_map);
        let mut graph = DiGraph::<u16, u64>::new();
        let mut node_map = HashMap::<u16, NodeIndex>::new();

        /*
        Determines whether candidate1 is preferred over candidate2 overall,
        or vice versa, or there is no net preference between the two.
        Also returns the net number of votes along said overall preference
        */
        let get_preference = |
            candidate1: u16, candidate2: u16
        | -> (PairPreferences, u64) {
            let preferred_over_votes =
                ranked_pairs_map.get(&(candidate1, candidate2))
                .unwrap_or(&0);
            let preferred_against_votes =
                ranked_pairs_map.get(&(candidate2, candidate1))
                .unwrap_or(&0);

            /*
            println!("C_PAIR {:?}", (
                (candidate1, candidate2),
                (preferred_over_votes, preferred_against_votes)
            ));
            */

            return if preferred_over_votes > preferred_against_votes {
                let strength = preferred_over_votes - preferred_against_votes;
                (PairPreferences::PreferredOver, strength)
            } else if preferred_over_votes == preferred_against_votes {
                (PairPreferences::Inconclusive, 0)
            } else {
                let strength = preferred_against_votes - preferred_over_votes;
                (PairPreferences::PreferredAgainst, strength)
            }
        };

        fn get_or_create_node (
            graph: &mut DiGraph<u16, u64>,
            node_map: &mut HashMap<u16, NodeIndex>,
            candidate: u16
        ) -> NodeIndex {
            // println!("NODE_MAP_PRE {:?}", (candidate, &node_map, &graph));
            let node = match node_map.get(&candidate) {
                Some(node) => { *node }
                None => {
                    let node = graph.add_node(candidate);
                    node_map.insert(candidate, node);
                    node
                }
            };

            // println!("NODE_MAP_POST {:?}", (candidate, &node_map, &graph));
            return node;
        }

        // initialize all the nodes in the graph
        for candidate in &candidates {
            get_or_create_node(&mut graph, &mut node_map, *candidate);
        }

        // construct preference strength graph between candidates
        for (candidate1, candidate2) in iproduct!(&candidates, &candidates) {
            if candidate1 == candidate2 { continue }
            let (preference, strength) =
                get_preference(*candidate1, *candidate2);

            match preference {
                PairPreferences::PreferredAgainst => { continue }
                PairPreferences::Inconclusive => { continue }
                PairPreferences::PreferredOver => {}
            }

            assert!(preference == PairPreferences::PreferredOver);
            let node1_idx =
                get_or_create_node(&mut graph, &mut node_map, *candidate1);
            let node2_idx =
                get_or_create_node(&mut graph, &mut node_map, *candidate2);
            if !graph.contains_edge(node1_idx, node2_idx) {
                // println!("ADD_EDGE {:?}", (node1_idx, node2_idx));
                graph.add_edge(node1_idx, node2_idx, strength);
            }
        }

        // println!("GRAPH {:?}", graph);
        // unable to establish pecking order among candidates
        if !(is_graph_acyclic(&graph) && is_graph_weakly_connected(&graph)) {
            /*
            println!("POST_RANK_FILTER {:?}", (
                &candidates, is_graph_acyclic(&graph),
                is_graph_weakly_connected(&graph))
            );
            */
            return candidates.clone();
        }

        let has_no_outgoing_edges = |&node: &NodeIndex| -> bool {
            graph.neighbors_directed(node, Direction::Outgoing).count() == 0
        };
        let weakest_nodes: Vec<NodeIndex> = graph
            .node_indices()
            .filter(has_no_outgoing_edges)
            .collect();

        let weakest_candidates = weakest_nodes
            .iter().map(|&index| graph[index]).collect();
        // println!("POST_NODES {:?}", weakest_nodes);
        // println!("POST_RANK_FILTER {:?}", weakest_candidates);
        return weakest_candidates;
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

    pub fn run_election(&self, votes: Vec<RankedVote>) -> Option<u16> {
        let mut rcv = RankedChoiceVoteTrie {
            root: Default::default(),
            dowdall_score_map: Default::default(),
            elimination_strategy: self.elimination_strategy.clone(),
            unique_candidates: Default::default()
        };
        rcv.insert_votes(votes);
        return rcv.determine_winner();
    }

    fn build_ranked_pairs_map(
        &self, node: &TrieNode, search_path: &mut Vec<u16>,
        ranked_pairs_map: &mut HashMap<(u16, u16), u64>,
        unique_candidates: &HashSet<u16>
    ) {
        let kv_pairs_vec: Vec<(Box<&VoteValues>, Box<&TrieNode>)> =
            node.children.iter().map(|(vote_value, node)| {
                (Box::new(vote_value), Box::new(node))
            }).collect();

        // number of votes that terminate at node
        let mut terminating_votes: u64 = node.num_votes;
        // println!("NODE_VOTES {:?}", node.num_votes);

        for (boxed_vote_value, boxed_child) in kv_pairs_vec {
            let vote_value = *boxed_vote_value;
            let child = *boxed_child;

            // println!("CHILD_VOTES {:?}", child.num_votes);
            assert!(terminating_votes >= child.num_votes);
            terminating_votes -= child.num_votes;

            let candidate = match vote_value {
                VoteValues::SpecialVote(_) => { continue }
                VoteValues::Candidate(candidate) => { candidate }
            };

            for preferable_candidate in search_path.iter() {
                let ranked_pair = (*preferable_candidate, *candidate);
                let pairwise_votes =
                    ranked_pairs_map.entry(ranked_pair).or_insert(0);
                *pairwise_votes += child.num_votes;
            }

            search_path.push(*candidate);
            self.build_ranked_pairs_map(
                child, search_path, ranked_pairs_map, unique_candidates
            );
            search_path.pop();
        };

        if terminating_votes > 0 {
            // println!("UNIQUE {:?}", unique_candidates);
            // println!("TERMINATE {:?}", (&search_path, terminating_votes));
            // candidates who weren't explicitly listed in current vote path
            let search_path: &Vec<u16> = search_path.as_ref();
            let mut unspecified_candidates = unique_candidates.clone();
            for candidate in search_path {
                unspecified_candidates.remove(candidate);
            }

            let pairs = iproduct!(search_path, &unspecified_candidates);
            for (preferable_candidate, candidate) in pairs {
                let ranked_pair = (*preferable_candidate, *candidate);
                let pairwise_votes =
                    ranked_pairs_map.entry(ranked_pair).or_insert(0);
                // println!("INSERT {:?}", (ranked_pair, terminating_votes));
                *pairwise_votes += terminating_votes;
            }
        }
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

        let mut ranked_pairs_map: HashMap<(u16, u16), u64> = HashMap::new();
        if self.elimination_strategy == EliminationStrategies::RankedPairs {
            self.build_ranked_pairs_map(
                &self.root, &mut Vec::new(), &mut ranked_pairs_map,
                &self.unique_candidates
            );
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
                    weakest_candidates = self.find_dowdall_weakest(
                        weakest_candidates
                    );
                },
                EliminationStrategies::RankedPairs => {
                    weakest_candidates = self.find_ranked_pairs_weakest(
                        weakest_candidates, &ranked_pairs_map
                    );
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
