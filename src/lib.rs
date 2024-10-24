use std::cmp::{min, Ordering, PartialEq};
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
pub struct TrieNode {
    children: HashMap<VoteValues, TrieNode>,
    num_votes: u64
}

impl TrieNode {
    pub fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            num_votes: 0,
        }
    }

    pub fn get_num_votes(&self) -> u64 {
        self.num_votes
    }

    pub fn search_or_create_child(
        &mut self, vote_value: VoteValues
    ) -> &mut TrieNode {
        self.children.entry(vote_value).or_default()
    }

    pub fn search_child(&self, vote_value: VoteValues) -> Option<&TrieNode> {
        if let Some(node_ref) = self.children.get(&vote_value) {
            Some(node_ref)
        } else {
            None
        }
    }
}

pub struct RankedChoiceVoteTrie {
    root: TrieNode,
    dowdall_score_map: HashMap<u32, f32>,
    elimination_strategy: EliminationStrategies,
    unique_candidates: HashSet<u32>
}

struct VoteTransfer<'a> {
    next_candidate: u32,
    next_node: &'a TrieNode,
    num_votes: u64
}

struct VoteTransferChanges<'a> {
    withhold_votes: u64, abstain_votes: u64,
    // (next candidate, next node, num votes to transfer to next candidate)
    vote_transfers: Vec<VoteTransfer<'a>>
}

// strategies for how to eliminate candidates each round
#[derive(Copy, Clone, PartialEq)]
pub enum EliminationStrategies {
    // removes all candidates with the lowest number of votes each round
    EliminateAll,
    // eliminate the candidate(s) with both the lowest number of votes
    // followed by the lowest dowdall score
    DowdallScoring,
    // eliminate the candidate(s) with both the lowest number of votes
    // and who lose against other candidates with the same number of votes
    // in a head-to-head comparison
    RankedPairs,
    // compare the candidate(s) that have the lowest and second-lowest number
    // of votes each round and eliminate the candidate(s) who lose to
    // to the other candidates in this group in a head-to-head comparison
    CondorcetRankedPairs
}

fn is_graph_acyclic(graph: &DiGraph<u32, u64>) -> bool {
    /*
    checks if there doesn't exist any path of directed edges
    from some edge in the graph back to itself
    */
    if graph.node_count() == 0 { return true }
    let nodes: Vec<NodeIndex> = graph.node_indices().collect();
    let mut all_explored_nodes = HashSet::<NodeIndex>::new();

    fn dfs_find_cycle(
        node: &NodeIndex, path: &mut Vec<NodeIndex>,
        explored: &mut HashSet<NodeIndex>, graph: &DiGraph<u32, u64>
    ) -> bool {
        // use DFS to see if a cycle can be created from paths starting from node
        explored.insert(*node);

        // get neighbors of node where there is an
        // outgoing edge from node to neighbor
        let directed_neighbors: Vec<NodeIndex> = graph
            .edges_directed(*node, Direction::Outgoing)
            .map(|edge| { edge.target()} )
            .collect();

        for neighbor in directed_neighbors {
            if path.contains(&neighbor) { return true }
            path.push(neighbor);
            let has_cycle = dfs_find_cycle(&neighbor, path, explored, graph);
            path.pop();

            if has_cycle { return true }
        }

        false
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

    true
}

fn is_graph_weakly_connected(graph: &DiGraph<u32, u64>) -> bool {
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
        neighbors
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

    explored_nodes.len() == graph.node_count()
}

impl Default for RankedChoiceVoteTrie {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn get_num_votes(&self) -> u64 {
        self.root.get_num_votes()
    }

    pub fn set_elimination_strategy(&mut self, strategy: EliminationStrategies) {
        self.elimination_strategy = strategy;
    }

    pub fn insert_votes(&mut self, votes: Vec<RankedVote>) {
        for vote in votes {
            self.insert_vote(vote);
        }
    }

    pub fn insert_vote(&mut self, vote: RankedVote) {
        self.root.num_votes += 1;
        let mut current = &mut self.root;
        let vote_items = vote.iter().enumerate();

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

    pub fn search_nodes(
        &mut self, ranked_vote: RankedVote
    ) -> Option<Vec<&TrieNode>> {
        // return path of trie nodes corresponding to ranked vote
        // returns None if there is no existing matching path in trie
        let mut current = &self.root;
        let vote_values: Vec<VoteValues> = ranked_vote.iter().collect();
        let mut node_path = vec![current];

        for vote_value in vote_values {
            let child_node = match current.children.get(&vote_value) {
                Some(node) => node,
                None => return None,
            };

            current = child_node;
            node_path.push(current);
        }
        Some(node_path)
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
                        next_candidate: *next_candidate, next_node,
                        num_votes: next_node.num_votes
                    });
                }
            }
        }

        transfer_changes
    }

    fn find_condorcet_ranked_pairs_weakest(
        &self, candidate_vote_counts: &HashMap<u32, u64>,
        ranked_pairs_map: &HashMap<(u32, u32), u64>,
        lowest_vote_candidates: Vec<u32>
    ) -> Vec<u32> {
        println!("CC_PRE_RANK_FILTER {:?}", candidate_vote_counts);
        println!("CC_PAIRS_MAP {:?}", ranked_pairs_map);
        let mut vote_counts: Vec<u64> =
            candidate_vote_counts.values().cloned().collect();
        vote_counts.sort();

        // get the second-lowest number of effective votes, or the lowest
        // number of votes if the second-lowest number of effective votes
        // is not available
        let vote_threshold = match vote_counts.get(1) {
            Some(second_lowest_votes) => { *second_lowest_votes }
            None => {
                match vote_counts.get(0) {
                    Some(lowest_votes) => { *lowest_votes }
                    None => { return vec![] }
                }
            }
        };

        // find candidates with less than or equal to the
        // second-lowest number of effective votes
        let mut weak_candidates: Vec<u32> = Vec::new();
        for (candidate, num_votes) in candidate_vote_counts {
            if *num_votes <= vote_threshold {
                weak_candidates.push(*candidate);
            }
        }

        let pairs_result = self.find_ranked_pairs_weakest(
            weak_candidates, ranked_pairs_map
        );

        if pairs_result.1 == false {
            lowest_vote_candidates
        } else {
            pairs_result.0
        }
    }

    fn find_ranked_pairs_weakest(
        &self, candidates: Vec<u32>,
        ranked_pairs_map: &HashMap<(u32, u32), u64>
    ) -> (Vec<u32>, bool) {
        /*
        Finds the candidates that perform the worst in pairwise
        head-to-head comparison.
        Returns the worst performing candidates, and whether it was possible
        to construct a preference graph
        */
        let mut graph = DiGraph::<u32, u64>::new();
        let mut node_map = HashMap::<u32, NodeIndex>::new();

        /*
        Determines whether candidate1 is preferred over candidate2 overall,
        or vice versa, or there is no net preference between the two.
        Also returns the net number of votes along said overall preference
        */
        let get_preference = |
            candidate1: u32, candidate2: u32
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

            match preferred_over_votes.cmp(preferred_against_votes) {
                Ordering::Greater => {
                    let strength =
                        preferred_over_votes - preferred_against_votes;
                    (PairPreferences::PreferredOver, strength)
                }
                Ordering::Equal => {
                    (PairPreferences::Inconclusive, 0)
                }
                Ordering::Less => {
                    let strength =
                        preferred_against_votes - preferred_over_votes;
                    (PairPreferences::PreferredAgainst, strength)
                }
            }
        };

        fn get_or_create_node (
            graph: &mut DiGraph<u32, u64>,
            node_map: &mut HashMap<u32, NodeIndex>,
            candidate: u32
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
            node
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
            return (candidates.clone(), false);
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
        (weakest_candidates, true)
    }

    fn find_dowdall_weakest(&self, candidates: Vec<u32>) -> Vec<u32> {
        /*
        returns the subset of candidates from the input candidates vector
        that score the lowest according the dowdall scoring criteria
        */
        let mut min_score = f32::MAX;
        let mut weakest_candidates: Vec<u32> = Vec::new();

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

        weakest_candidates
    }

    pub fn run_election(&self, votes: Vec<RankedVote>) -> Option<u32> {
        let mut rcv = RankedChoiceVoteTrie {
            root: Default::default(),
            dowdall_score_map: Default::default(),
            elimination_strategy: self.elimination_strategy.clone(),
            unique_candidates: Default::default()
        };
        rcv.insert_votes(votes);
        rcv.determine_winner()
    }

    fn build_ranked_pairs_map(
        node: &TrieNode, search_path: &mut Vec<u32>,
        ranked_pairs_map: &mut HashMap<(u32, u32), u64>,
        unique_candidates: &HashSet<u32>
    ) {
        let kv_pairs_vec: Vec<(&VoteValues, &TrieNode)> =
            node.children.iter().map(|(vote_value, node)| {
                (vote_value, node)
            }).collect();

        // number of votes that terminate at node
        let mut terminating_votes: u64 = node.num_votes;
        // println!("NODE_VOTES {:?}", node.num_votes);

        for (vote_value, child) in kv_pairs_vec {
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
            Self::build_ranked_pairs_map(
                child, search_path, ranked_pairs_map, unique_candidates
            );
            search_path.pop();
        };

        if terminating_votes > 0 {
            // println!("UNIQUE {:?}", unique_candidates);
            // println!("TERMINATE {:?}", (&search_path, terminating_votes));
            // candidates who weren't explicitly listed in current vote path
            let search_path: &Vec<u32> = search_path;
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

    pub fn determine_winner(&self) -> Option<u32> {
        // println!("RUN_ELECTION_START");
        let mut candidate_vote_counts: HashMap<u32, u64> = HashMap::new();
        let mut frontier_nodes:
            HashMap<u32, Vec<&TrieNode>> = HashMap::new();
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
                    frontier_nodes.insert(*candidate, vec![node]);
                    total_candidate_votes += node.num_votes;
                    effective_total_votes += node.num_votes;
                }
            };
        }

        let mut ranked_pairs_map: HashMap<(u32, u32), u64> = HashMap::new();
        let strategy = self.elimination_strategy;
        if
            (strategy == EliminationStrategies::RankedPairs) ||
            (strategy == EliminationStrategies::CondorcetRankedPairs)
        {
            Self::build_ranked_pairs_map(
                &self.root, &mut Vec::new(), &mut ranked_pairs_map,
                &self.unique_candidates
            );
        }

        while !candidate_vote_counts.is_empty() {
            let mut min_candidate_votes: u64 = u64::MAX;
            // impossible for any candidate to win as sum of
            // candidate votes is under the total number of votes cast
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
            let mut lowest_vote_candidates: Vec<u32> = Vec::new();
            for (candidate, num_votes) in &candidate_vote_counts {
                if *num_votes == min_candidate_votes {
                    lowest_vote_candidates.push(*candidate);
                }
            }

            // further filter down candidates to eliminate using
            // specified elimination strategy
            let weakest_candidates = match self.elimination_strategy {
                EliminationStrategies::EliminateAll => {
                    lowest_vote_candidates
                },
                EliminationStrategies::DowdallScoring => {
                    self.find_dowdall_weakest(lowest_vote_candidates)
                },
                EliminationStrategies::RankedPairs => {
                    self.find_ranked_pairs_weakest(
                        lowest_vote_candidates, &ranked_pairs_map
                    ).0
                },
                EliminationStrategies::CondorcetRankedPairs => {
                    self.find_condorcet_ranked_pairs_weakest(
                        &candidate_vote_counts, &ranked_pairs_map,
                        lowest_vote_candidates
                    )
                }
            };

            // find all candidates, nodes, and vote counts to transfer to
            let mut all_vote_transfers: Vec<VoteTransfer> = Vec::new();
            let mut new_withhold_votes: u64 = 0;
            let mut new_abstain_votes: u64 = 0;

            for weakest_candidate in weakest_candidates {
                let candidate_nodes = frontier_nodes.get(&weakest_candidate)
                    .expect("all uneliminated candidates must have node(s)");

                for node in candidate_nodes {
                    let transfer_result = self.transfer_next_votes(node);
                    new_abstain_votes += transfer_result.abstain_votes;
                    new_withhold_votes += transfer_result.withhold_votes;
                    all_vote_transfers.extend(transfer_result.vote_transfers);
                }

                candidate_vote_counts.remove(&weakest_candidate);
                frontier_nodes.remove(&weakest_candidate);
            }

            // 0 vote transfers will be done, election is unable to progress
            if all_vote_transfers.is_empty() { return None; }
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
                    .entry(next_candidate).or_default();

                *next_candidate_votes += vote_allocation;
                next_candidate_nodes.push(vote_transfer.next_node);
            }
        }

        None
    }
}
