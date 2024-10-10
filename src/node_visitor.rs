use std::fmt::Display;

use crate::{node::Node, stats_producer::StatsProducer};

pub trait NodeVisitor {
    fn visit(&mut self, node: &Node);
}

#[derive(Default, Debug)]
pub struct StatsNodeVisitor {
    all_stats: serde_json::Value,
}

impl Display for StatsNodeVisitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{:#}", self.all_stats)
        } else {
            write!(f, "{}", self.all_stats)
        }
    }
}

impl NodeVisitor for StatsNodeVisitor {
    fn visit(&mut self, node: &Node) {
        if let Some(stats) = node.get_stats() {
            self.all_stats[node.name()] = stats;
        }
    }
}
