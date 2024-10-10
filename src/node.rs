use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use serde_json::json;

use crate::{
    node_visitor::NodeVisitor, packet_handler::SomePacketHandler, stats_producer::StatsProducer,
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
pub struct NodeRef<T>(Arc<Mutex<Node<T>>>);

// derive(Clone) doesn't work with the generic
impl<T> Clone for NodeRef<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> NodeRef<T> {
    pub fn new(node: Node<T>) -> NodeRef<T> {
        NodeRef(Arc::new(Mutex::new(node)))
    }

    pub fn name(&self) -> String {
        self.0.lock().unwrap().name.clone()
    }

    pub fn set_next(&self, next: NodeRef<T>) {
        self.0.lock().unwrap().next = Some(next)
    }

    pub fn set_prev(&self, prev: NodeRef<T>) {
        self.0.lock().unwrap().prev = Some(prev)
    }

    pub fn process_packet(&self, packet: T) {
        self.0.lock().unwrap().process_packet(packet);
    }

    pub fn visit(&self, visitor: &mut dyn NodeVisitor<T>) {
        self.0.lock().unwrap().visit(visitor)
    }
}

/// A helper type to model the possible outcomes of passing a packet to [`SomePacketHandler`].
// Note: I had a thought to get rid of 'ForwardToNext' and always use 'ForwardTo', but non-demux
// nodes don't know/care if there _is_ a next node.  For a demuxer, which uses ForwardTo, there
// will be a definitive next node to forward to, so at this point I think the two variants make
// sense?
enum SomePacketHandlerResult<'a, T> {
    // The given PacketInfo should be forwarded to the next node
    ForwardToNext(T),
    // The given PacketInfo should be forwarded to the given node
    ForwardTo(T, &'a NodeRef<T>),
    // The PacketInfo was consumed
    Consumed,
    // The PacketInfo should be discarded
    Discard,
}

pub struct Node<T> {
    name: String,
    handler: SomePacketHandler<T>,
    stats: NodeStatsTracker,
    next: Option<NodeRef<T>>,
    prev: Option<NodeRef<T>>,
}

impl<T> Node<T> {
    pub fn new<U: Into<String>, V: Into<SomePacketHandler<T>>>(name: U, handler: V) -> Node<T> {
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

    pub fn set_next(&mut self, next: NodeRef<T>) {
        self.next = Some(next)
    }

    pub fn set_prev(&mut self, prev: NodeRef<T>) {
        self.prev = Some(prev)
    }

    pub fn process_packet(&mut self, packet: T) {
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

    pub fn visit(&mut self, visitor: &mut dyn NodeVisitor<T>) {
        visitor.visit(self);
        if let Some(ref mut n) = self.next {
            n.visit(visitor);
        }
    }
}

impl<T> StatsProducer for Node<T> {
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
