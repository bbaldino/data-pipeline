use std::marker::PhantomData;

use crate::{
    node::{Node, NodeRef},
    packet_handler::{PacketDemuxer, SomePacketHandler},
};

pub trait PipelineState {}
pub struct Open;
pub struct Terminated;

impl PipelineState for Open {}
impl PipelineState for Terminated {}

#[derive(Default)]
pub struct PipelineBuilder<T: PipelineState> {
    nodes: Vec<NodeRef>,
    _state: PhantomData<T>,
}

impl Default for PipelineBuilder<Open> {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineBuilder<Open> {
    pub fn new() -> Self {
        PipelineBuilder::<Open> {
            nodes: Vec::new(),
            _state: PhantomData,
        }
    }
}

impl<T: PipelineState> PipelineBuilder<T> {
    pub fn build(mut self) -> NodeRef {
        self.nodes.remove(0)
    }
}

impl PipelineBuilder<Open> {
    pub fn attach(mut self, node: NodeRef) -> Self {
        if let Some(last) = self.nodes.last() {
            last.set_next(node.clone());
            node.set_prev(last.clone());
        }
        self.nodes.push(node);
        self
    }

    pub fn demux<T: Into<String>>(
        mut self,
        name: T,
        demuxer: impl PacketDemuxer + Send + 'static,
    ) -> PipelineBuilder<Terminated> {
        let new_node = NodeRef::new(Node::new(
            name,
            SomePacketHandler::Demuxer(Box::new(demuxer)),
        ));
        if let Some(last) = self.nodes.last() {
            last.set_next(new_node.clone());
            new_node.set_prev(last.clone());
        }
        self.nodes.push(new_node);
        // Demuxers can't be attached to like normal nodes because they handle their own downstream
        // paths, so although a pipeline will almost always continue after a demuxer, from a
        // builder perspective the demuxer needs to be built with its own sub-pipelines for its
        // downstream paths.
        PipelineBuilder::<Terminated> {
            nodes: self.nodes,
            _state: PhantomData::<Terminated>,
        }
    }
}
