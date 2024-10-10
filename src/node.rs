use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use serde_json::json;

use crate::{
    node_visitor::NodeVisitor, packet_handler::SomePacketHandler, packet_info::PacketInfo,
    stats_producer::StatsProducer,
};

#[derive(Default)]
struct NodeStatsTracker {
    packets_ingress: u32,
    packets_egress: u32,
    packets_discarded: u32,
    errors: u32,
    total_processing_time: Duration,
}

/// [`NodeRef`] is really just a convenience helper to hide how references are held and take care
/// of the mutex logic
#[derive(Clone)]
pub struct NodeRef(Arc<Mutex<Node>>);

impl NodeRef {
    pub fn new(node: Node) -> Self {
        NodeRef(Arc::new(Mutex::new(node)))
    }

    pub fn name(&self) -> String {
        self.0.lock().unwrap().name.clone()
    }

    pub fn set_next(&self, next: NodeRef) {
        self.0.lock().unwrap().next = Some(next)
    }

    pub fn set_prev(&self, prev: NodeRef) {
        self.0.lock().unwrap().prev = Some(prev)
    }

    pub fn process_packet(&self, packet: PacketInfo) {
        self.0.lock().unwrap().process_packet(packet);
    }

    pub fn visit(&self, visitor: &mut dyn NodeVisitor) {
        self.0.lock().unwrap().visit(visitor)
    }
}

/// A helper type to model the possible outcomes of passing a packet to [`SomePacketHandler`].
// Note: I had a thought to get rid of 'ForwardToNext' and always use 'ForwardTo', but non-demux
// nodes don't know/care if there _is_ a next node.  For a demuxer, which uses ForwardTo, there
// will be a definitive next node to forward to, so at this point I think the two variants make
// sense?
enum SomePacketHandlerResult<'a> {
    // The given PacketInfo should be forwarded to the next node
    ForwardToNext(PacketInfo),
    // The given PacketInfo should be forwarded to the given node
    ForwardTo(PacketInfo, &'a NodeRef),
    // The PacketInfo was consumed
    Consumed,
    // The PacketInfo should be discarded
    Discard,
}

pub struct Node {
    name: String,
    handler: SomePacketHandler,
    stats: NodeStatsTracker,
    next: Option<NodeRef>,
    prev: Option<NodeRef>,
}

impl Node {
    pub fn new<T: Into<String>, U: Into<SomePacketHandler>>(name: T, handler: U) -> Self {
        Self {
            name: name.into(),
            handler: handler.into(),
            stats: NodeStatsTracker::default(),
            next: None,
            prev: None,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn set_next(&mut self, next: NodeRef) {
        self.next = Some(next)
    }

    pub fn set_prev(&mut self, prev: NodeRef) {
        self.prev = Some(prev)
    }

    pub fn process_packet(&mut self, packet: PacketInfo) {
        self.stats.packets_ingress += 1;
        let start = Instant::now();
        let packet_handler_result = match self.handler {
            SomePacketHandler::Observer(ref mut o) => {
                o.observe(&packet);
                Ok(SomePacketHandlerResult::ForwardToNext(packet))
            }
            SomePacketHandler::Transformer(ref mut t) => match t.transform(packet) {
                Ok(transformed) => Ok(SomePacketHandlerResult::ForwardToNext(transformed)),
                Err(e) => Err(anyhow!("Packet transformer {} failed: {e:?}", self.name)),
            },
            SomePacketHandler::Filter(ref mut f) => match f.should_forward(&packet) {
                true => Ok(SomePacketHandlerResult::ForwardToNext(packet)),
                false => Ok(SomePacketHandlerResult::Discard),
            },
            SomePacketHandler::Consumer(ref mut c) => {
                c.consume(packet);
                Ok(SomePacketHandlerResult::Consumed)
            }
            SomePacketHandler::Demuxer(ref mut d) => {
                if let Some(path) = d.find_path(&packet) {
                    Ok(SomePacketHandlerResult::ForwardTo(packet, path))
                } else {
                    Ok(SomePacketHandlerResult::Discard)
                }
            }
        };
        let processing_duration = Instant::now() - start;
        self.stats.total_processing_time += processing_duration;
        match packet_handler_result {
            Ok(SomePacketHandlerResult::ForwardToNext(p)) => {
                self.stats.packets_egress += 1;
                if let Some(ref n) = self.next {
                    n.process_packet(p);
                }
            }
            Ok(SomePacketHandlerResult::ForwardTo(p, next)) => {
                self.stats.packets_egress += 1;
                next.process_packet(p);
            }
            Ok(SomePacketHandlerResult::Discard) => {
                self.stats.packets_discarded += 1;
            }
            Ok(SomePacketHandlerResult::Consumed) => {
                // no-op
            }
            Err(e) => {
                self.stats.errors += 1;
                println!("Error processing packet: {e:?}")
            }
        }
    }

    pub fn visit(&mut self, visitor: &mut dyn NodeVisitor) {
        visitor.visit(self);
        if let Some(ref mut n) = self.next {
            n.visit(visitor);
        }
    }
}

impl StatsProducer for Node {
    fn get_stats(&self) -> Option<serde_json::Value> {
        Some(json!({
            "packets_ingress": self.stats.packets_ingress,
            "packets_egress": self.stats.packets_egress,
            "packets_discarded": self.stats.packets_discarded,
            "errors": self.stats.errors,
            "total processing time": format!("{:?}", self.stats.total_processing_time),
            "process time per packet": format!("{:?}", (self.stats.total_processing_time / self.stats.packets_ingress)),
            "handler_stats": self.handler.get_stats(),
        }))
    }
}
