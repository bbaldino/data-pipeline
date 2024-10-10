use crate::{
    data_handler::{DataDemuxer, SomeDataHandler},
    node::NodeRef,
    node_visitor::NodeVisitor,
    stats_producer::StatsProducer,
};

pub type PathPredicate<T> = dyn Fn(&T) -> bool + Send + 'static;

pub struct ConditionalPath<T, U> {
    pub predicate: Box<PathPredicate<T>>,
    pub next: U,
}

/// A Demuxer whose paths are fixed and known at creation time, so that they don't need to be
/// locked for each data item.
#[derive(Default)]
pub struct StaticDemuxer<T> {
    paths: Vec<ConditionalPath<T, NodeRef<T>>>,
}

impl<T> StatsProducer for StaticDemuxer<T> {}

impl<T> StaticDemuxer<T> {
    pub fn new(paths: Vec<ConditionalPath<T, NodeRef<T>>>) -> Self {
        Self { paths }
    }
}

impl<T> DataDemuxer<T> for StaticDemuxer<T> {
    fn find_path(&mut self, data: &T) -> Option<&NodeRef<T>> {
        for path in &self.paths {
            if (path.predicate)(data) {
                return Some(&path.next);
            }
        }
        None
    }

    fn visit(&mut self, visitor: &mut dyn NodeVisitor<T>) {
        for path in &self.paths {
            path.next.visit(visitor);
        }
    }
}

impl<T> From<StaticDemuxer<T>> for SomeDataHandler<T>
where
    T: 'static,
{
    fn from(value: StaticDemuxer<T>) -> Self {
        SomeDataHandler::Demuxer(Box::new(value))
    }
}
