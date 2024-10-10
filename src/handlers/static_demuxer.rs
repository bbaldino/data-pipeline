use crate::{
    node::NodeRef,
    packet_handler::{PacketDemuxer, SomePacketHandler},
    stats_producer::StatsProducer,
};

pub type PathPredicate<T> = dyn Fn(&T) -> bool + Send + 'static;

pub struct ConditionalPath<T, U> {
    pub predicate: Box<PathPredicate<T>>,
    pub next: U,
}

/// A Demuxer whose paths are fixed and known at creation time, so that they don't need to be
/// locked for each packet.
#[derive(Default)]
pub struct StaticDemuxer<T> {
    packet_paths: Vec<ConditionalPath<T, NodeRef<T>>>,
}

impl<T> StatsProducer for StaticDemuxer<T> {}

impl<T> StaticDemuxer<T> {
    pub fn new(packet_paths: Vec<ConditionalPath<T, NodeRef<T>>>) -> Self {
        Self { packet_paths }
    }
}

impl<T> PacketDemuxer<T> for StaticDemuxer<T> {
    fn find_path(&mut self, packet_info: &T) -> Option<&NodeRef<T>> {
        for path in &self.packet_paths {
            if (path.predicate)(packet_info) {
                return Some(&path.next);
            }
        }
        None
    }
}

impl<T> From<StaticDemuxer<T>> for SomePacketHandler<T>
where
    T: 'static,
{
    fn from(value: StaticDemuxer<T>) -> Self {
        SomePacketHandler::Demuxer(Box::new(value))
    }
}

// impl_conversion_to_some_packet_handler!(StaticDemuxer, Demuxer);
