pub mod data_handler;
pub mod handlers;
pub mod node;
pub mod node_visitor;
pub mod pipeline_builder;
pub mod stats_producer;

#[cfg(test)]
mod test {
    use std::time::Instant;

    use serde_json::json;

    use crate::{
        data_handler::DataObserver,
        handlers::static_demuxer::{ConditionalPath, StaticDemuxer},
        impl_conversion_to_some_data_handler,
        node::{Node, NodeRef},
        node_visitor::StatsNodeVisitor,
        pipeline_builder::PipelineBuilder,
        stats_producer::StatsProducer,
    };

    struct DataLogger;

    impl StatsProducer for DataLogger {
        // Just an example of handler-specific stats
        fn get_stats(&self) -> Option<serde_json::Value> {
            Some(json!({
                "num_prints": "a lot",
            }))
        }
    }

    impl<T> DataObserver<T> for DataLogger {
        fn observe(&mut self, _data: &T) {
            // println!("saw item {data:?}");
        }
    }

    impl_conversion_to_some_data_handler!(DataLogger, Observer);

    #[test]
    fn test() {
        let num_items = 1000000;
        let num_nodes = 10;
        // let first_node = NodeRef::new(Node::new("1"));
        // let mut prev_node = first_node.clone();
        let mut builder = PipelineBuilder::default();
        for i in 0..num_nodes {
            builder = builder.attach(NodeRef::new(Node::new(format!("{i}"), DataLogger)));
        }

        let first_node = builder.build();
        let start = Instant::now();
        for i in 0..num_items {
            first_node.process_data(i);
        }
        let duration = Instant::now() - start;
        println!(
            "{num_nodes} nodes processed {num_items} items in {}ms ({} items/msec)",
            duration.as_millis(),
            (num_items as u128 / duration.as_millis())
        );
        let mut stats = StatsNodeVisitor::default();
        first_node.visit(&mut stats);
        println!("{:#}", stats);
    }

    #[test]
    fn test_builder() {
        let pipeline = PipelineBuilder::new()
            .demux(
                "odd/even demuxer",
                StaticDemuxer::new(vec![
                    ConditionalPath {
                        predicate: Box::new(|num: &u32| num % 2 == 0),
                        next: PipelineBuilder::new()
                            .attach(NodeRef::new(Node::new("1a", DataLogger)))
                            .attach(NodeRef::new(Node::new("2a", DataLogger)))
                            .attach(NodeRef::new(Node::new("3a", DataLogger)))
                            .build(),
                    },
                    ConditionalPath {
                        predicate: Box::new(|num: &u32| num % 2 == 1),
                        next: PipelineBuilder::new()
                            .attach(NodeRef::new(Node::new("1b", DataLogger)))
                            .attach(NodeRef::new(Node::new("2b", DataLogger)))
                            .attach(NodeRef::new(Node::new("3b", DataLogger)))
                            .build(),
                    },
                ]),
            )
            .build();

        for i in 0..10 {
            pipeline.process_data(i);
        }
        let mut stats = StatsNodeVisitor::default();
        pipeline.visit(&mut stats);
        println!("{:#}", stats);
    }
}
