pub mod handlers;
pub mod node;
pub mod node_visitor;
pub mod packet_handler;
pub mod packet_info;
pub mod pipeline_builder;
pub mod stats_producer;

#[cfg(test)]
mod test {
    use std::time::Instant;

    use serde_json::json;

    use crate::{
        handlers::static_demuxer::{ConditionalPath, StaticDemuxer},
        impl_conversion_to_some_packet_handler,
        node::{Node, NodeRef},
        node_visitor::StatsNodeVisitor,
        packet_handler::PacketObserver,
        packet_info::PacketInfo,
        pipeline_builder::PipelineBuilder,
        stats_producer::StatsProducer,
    };

    struct PacketLogger;

    impl StatsProducer for PacketLogger {
        // Just an example of handler-specific stats
        fn get_stats(&self) -> Option<serde_json::Value> {
            Some(json!({
                "num_prints": "a lot",
            }))
        }
    }

    impl<T> PacketObserver<T> for PacketLogger {
        fn observe(&mut self, _data: &T) {
            // println!("saw packet {data:?}");
        }
    }

    impl_conversion_to_some_packet_handler!(PacketLogger, Observer);

    #[test]
    fn test() {
        let num_packets = 1000000;
        let num_nodes = 10;
        // let first_node = NodeRef::new(Node::new("1"));
        // let mut prev_node = first_node.clone();
        let mut builder = PipelineBuilder::default();
        for i in 0..num_nodes {
            builder = builder.attach(NodeRef::new(Node::new(format!("{i}"), PacketLogger)));
        }

        let first_node = builder.build();
        let start = Instant::now();
        for _ in 0..num_packets {
            first_node.process_packet(PacketInfo);
        }
        let duration = Instant::now() - start;
        println!(
            "{num_nodes} nodes processed {num_packets} packets in {}ms ({} packets/msec)",
            duration.as_millis(),
            (num_packets as u128 / duration.as_millis())
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
                            .attach(NodeRef::new(Node::new("1a", PacketLogger)))
                            .attach(NodeRef::new(Node::new("2a", PacketLogger)))
                            .attach(NodeRef::new(Node::new("3a", PacketLogger)))
                            .build(),
                    },
                    ConditionalPath {
                        predicate: Box::new(|num: &u32| num % 2 == 1),
                        next: PipelineBuilder::new()
                            .attach(NodeRef::new(Node::new("1b", PacketLogger)))
                            .attach(NodeRef::new(Node::new("2b", PacketLogger)))
                            .attach(NodeRef::new(Node::new("3b", PacketLogger)))
                            .build(),
                    },
                ]),
            )
            .build();

        for i in 0..10 {
            pipeline.process_packet(i);
        }
        let mut stats = StatsNodeVisitor::default();
        pipeline.visit(&mut stats);
        println!("{:#}", stats);
    }
}
