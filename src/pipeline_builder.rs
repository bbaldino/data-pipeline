use std::marker::PhantomData;

use crate::{
    data_handler::{DataDemuxer, SomeDataHandler},
    node::{Node, NodeRef},
};

pub trait PipelineState {}
pub struct Open;
pub struct Terminated;

impl PipelineState for Open {}
impl PipelineState for Terminated {}

#[derive(Default)]
pub struct PipelineBuilder<S: PipelineState, T> {
    nodes: Vec<NodeRef<T>>,
    _state: PhantomData<S>,
}

impl<T> Default for PipelineBuilder<Open, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PipelineBuilder<Open, T> {
    pub fn new() -> Self {
        PipelineBuilder::<Open, T> {
            nodes: Vec::new(),
            _state: PhantomData,
        }
    }
}

impl<S: PipelineState, T> PipelineBuilder<S, T> {
    pub fn build(mut self) -> NodeRef<T> {
        self.nodes.remove(0)
    }
}

impl<T> PipelineBuilder<Open, T> {
    pub fn attach(mut self, node: NodeRef<T>) -> Self {
        if let Some(last) = self.nodes.last() {
            last.set_next(node.clone());
            node.set_prev(last.clone());
        }
        self.nodes.push(node);
        self
    }

    pub fn demux<U: Into<String>>(
        mut self,
        name: U,
        demuxer: impl DataDemuxer<T> + Send + 'static,
    ) -> PipelineBuilder<Terminated, T> {
        let new_node = NodeRef::new(Node::new(name, SomeDataHandler::Demuxer(Box::new(demuxer))));
        if let Some(last) = self.nodes.last() {
            last.set_next(new_node.clone());
            new_node.set_prev(last.clone());
        }
        self.nodes.push(new_node);
        // Demuxers can't be attached to like normal nodes because they handle their own downstream
        // paths, so although a pipeline will almost always continue after a demuxer, from a
        // builder perspective the demuxer needs to be built with its own sub-pipelines for its
        // downstream paths.
        PipelineBuilder::<Terminated, T> {
            nodes: self.nodes,
            _state: PhantomData::<Terminated>,
        }
    }
}
