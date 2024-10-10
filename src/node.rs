use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use serde_json::json;

use crate::{
    data_handler::SomeDataHandler, node_visitor::NodeVisitor, stats_producer::StatsProducer,
};

#[derive(Default)]
struct NodeStatsTracker {
    data_ingress: u32,
    data_egress: u32,
    data_discarded: u32,
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

impl<T> From<Node<T>> for NodeRef<T> {
    fn from(val: Node<T>) -> Self {
        NodeRef::new(val)
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

    pub fn process_data(&self, data: T) {
        self.0.lock().unwrap().process_data(data);
    }

    pub fn visit(&self, visitor: &mut dyn NodeVisitor<T>) {
        self.0.lock().unwrap().visit(visitor)
    }
}

/// A helper type to model the possible outcomes of passing data to [`SomeDataHandler`].
// Note: I had a thought to get rid of 'ForwardToNext' and always use 'ForwardTo', but non-demux
// nodes don't know/care if there _is_ a next node.  For a demuxer, which uses ForwardTo, there
// will be a definitive next node to forward to, so at this point I think the two variants make
// sense?
enum SomeDataHandlerResult<'a, T> {
    // The given data should be forwarded to the next node
    ForwardToNext(T),
    // The given data should be forwarded to the given node
    ForwardTo(T, &'a NodeRef<T>),
    // The data was consumed
    Consumed,
    // The data should be discarded
    Discard,
}

pub struct Node<T> {
    name: String,
    handler: SomeDataHandler<T>,
    stats: NodeStatsTracker,
    next: Option<NodeRef<T>>,
    prev: Option<NodeRef<T>>,
}

impl<T> Node<T> {
    pub fn new<U: Into<String>, V: Into<SomeDataHandler<T>>>(name: U, handler: V) -> Node<T> {
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

    pub fn process_data(&mut self, data: T) {
        self.stats.data_ingress += 1;
        let start = Instant::now();
        let data_handler_result = match self.handler {
            SomeDataHandler::Observer(ref mut o) => {
                o.observe(&data);
                Ok(SomeDataHandlerResult::ForwardToNext(data))
            }
            SomeDataHandler::Transformer(ref mut t) => match t.transform(data) {
                Ok(transformed) => Ok(SomeDataHandlerResult::ForwardToNext(transformed)),
                Err(e) => Err(anyhow!("Data transformer {} failed: {e:?}", self.name)),
            },
            SomeDataHandler::Filter(ref mut f) => match f.should_forward(&data) {
                true => Ok(SomeDataHandlerResult::ForwardToNext(data)),
                false => Ok(SomeDataHandlerResult::Discard),
            },
            SomeDataHandler::Consumer(ref mut c) => {
                c.consume(data);
                Ok(SomeDataHandlerResult::Consumed)
            }
            SomeDataHandler::Demuxer(ref mut d) => {
                if let Some(path) = d.find_path(&data) {
                    Ok(SomeDataHandlerResult::ForwardTo(data, path))
                } else {
                    Ok(SomeDataHandlerResult::Discard)
                }
            }
        };
        let processing_duration = Instant::now() - start;
        self.stats.total_processing_time += processing_duration;
        match data_handler_result {
            Ok(SomeDataHandlerResult::ForwardToNext(p)) => {
                self.stats.data_egress += 1;
                if let Some(ref n) = self.next {
                    n.process_data(p);
                }
            }
            Ok(SomeDataHandlerResult::ForwardTo(p, next)) => {
                self.stats.data_egress += 1;
                next.process_data(p);
            }
            Ok(SomeDataHandlerResult::Discard) => {
                self.stats.data_discarded += 1;
            }
            Ok(SomeDataHandlerResult::Consumed) => {
                // no-op
            }
            Err(e) => {
                self.stats.errors += 1;
                println!("Error processing data: {e:?}")
            }
        }
    }

    pub fn visit(&mut self, visitor: &mut dyn NodeVisitor<T>) {
        visitor.visit(self);
        if let SomeDataHandler::Demuxer(ref mut d) = self.handler {
            d.visit(visitor)
        };
        if let Some(ref mut n) = self.next {
            n.visit(visitor);
        }
    }
}

impl<T> StatsProducer for Node<T> {
    fn get_stats(&self) -> Option<serde_json::Value> {
        Some(json!({
            "data_ingress": self.stats.data_ingress,
            "data_egress": self.stats.data_egress,
            "data_discarded": self.stats.data_discarded,
            "errors": self.stats.errors,
            "total processing time": format!("{:?}", self.stats.total_processing_time),
            "process time per item": format!("{:?}", (self.stats.total_processing_time / self.stats.data_ingress)),
            "handler_stats": self.handler.get_stats(),
        }))
    }
}
