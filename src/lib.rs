use std::sync::{Arc, Mutex};

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

    pub fn process_packet(&self, packet: u32) {
        self.0.lock().unwrap().process_packet(packet);
    }
}

pub struct Node {
    name: String,
    next: Option<NodeRef>,
    prev: Option<NodeRef>,
}

impl Node {
    pub fn new<T: Into<String>>(name: T) -> Self {
        Self {
            name: name.into(),
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

    pub fn process_packet(&mut self, packet: u32) {
        // println!("node {} processing packet {packet}", self.name);
        if let Some(ref next) = self.next {
            next.process_packet(packet);
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Instant;

    use crate::{Node, NodeRef};

    fn connect(left: &NodeRef, right: &NodeRef) {
        left.set_next(right.clone());
        right.set_prev(left.clone())
    }

    #[test]
    fn test() {
        let num_packets = 1000000;
        let num_nodes = 10;
        let first_node = NodeRef::new(Node::new("1"));
        let mut prev_node = first_node.clone();
        for i in 1..num_nodes {
            let node = NodeRef::new(Node::new(format!("{i}")));
            connect(&prev_node, &node);
            prev_node = node;
        }

        let start = Instant::now();
        for i in 0..num_packets {
            first_node.process_packet(i);
        }
        let duration = Instant::now() - start;
        println!(
            "{num_nodes} nodes processed {num_packets} packets in {}ms ({} packets/msec)",
            duration.as_millis(),
            (num_packets as u128 / duration.as_millis())
        );
    }
}
