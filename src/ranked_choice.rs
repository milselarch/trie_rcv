use petgraph::graph::DiGraph;
use petgraph::prelude::*;

use crate::vote;
use vote::VoteValues;

/*
edges from (level, choice) to (next_level, next_choice)
edge weight is number of votes with edge transition
*/

struct RankedChoiceVoteGraph {
    graph: DiGraph<(u64, VoteValues), u64>
}

impl RankedChoiceVoteGraph {
    fn add_vote() {}
}