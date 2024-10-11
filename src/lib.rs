pub mod data_handler;
pub mod handlers;
pub mod node;
pub mod node_visitor;
pub mod pipeline_builder;
pub mod stats_producer;

#[cfg(test)]
mod test {
    use std::{fmt::Debug, time::Instant};

    use serde_json::json;

    use crate::{
        data_handler::{DataObserver, SomeDataHandler},
        handlers::static_demuxer::{ConditionalPath, StaticDemuxer},
        node::Node,
        node_visitor::StatsNodeVisitor,
        pipeline_builder::PipelineBuilder,
    };

    #[derive(Default)]
    struct DataLogger {
        num_logs: u32,
    }

    impl<T> DataObserver<T> for DataLogger
    where
        T: Debug,
    {
        fn observe(&mut self, data: &T) {
            self.num_logs += 1;
            println!("{data:?}")
        }

        fn get_stats(&self) -> Option<serde_json::Value> {
            Some(json!({
                "num_logs": self.num_logs,
            }))
        }
    }

    impl<T> From<DataLogger> for SomeDataHandler<T>
    where
        T: Debug,
    {
        fn from(val: DataLogger) -> Self {
            SomeDataHandler::Observer(Box::new(val))
        }
    }

    // impl_conversion_to_some_data_handler!(DataLogger, Observer);

    #[test]
    fn test() {
        let num_items = 1000000;
        let num_nodes = 10;
        // let first_node = NodeRef::new(Node::new("1"));
        // let mut prev_node = first_node.clone();
        let mut builder = PipelineBuilder::default();
        for i in 0..num_nodes {
            builder = builder.attach(Node::new(format!("{i}"), DataLogger::default()));
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
                            .attach(Node::new("1a", DataLogger::default()))
                            .attach(Node::new("2a", DataLogger::default()))
                            .attach(Node::new("3a", DataLogger::default()))
                            .build(),
                    },
                    ConditionalPath {
                        predicate: Box::new(|num: &u32| num % 2 == 1),
                        next: PipelineBuilder::new()
                            .attach(Node::new("1b", DataLogger::default()))
                            .attach(Node::new("2b", DataLogger::default()))
                            .attach(Node::new("3b", DataLogger::default()))
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
