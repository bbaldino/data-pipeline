use crate::{
    impl_conversion_to_some_packet_handler, node::NodeRef, packet_handler::PacketDemuxer,
    packet_info::PacketInfo, stats_producer::StatsProducer,
};

pub type PathPredicate<T> = dyn Fn(&T) -> bool + Send + 'static;

pub struct ConditionalPath<T, U> {
    pub predicate: Box<PathPredicate<T>>,
    pub next: U,
}

/// A Demuxer whose paths are fixed and known at creation time, so that they don't need to be
/// locked for each packet.
#[derive(Default)]
pub struct StaticDemuxer {
    packet_paths: Vec<ConditionalPath<PacketInfo, NodeRef>>,
}

impl StatsProducer for StaticDemuxer {}

impl StaticDemuxer {
    pub fn new<T: Into<ConditionalPath<PacketInfo, NodeRef>>>(packet_paths: Vec<T>) -> Self {
        Self {
            packet_paths: packet_paths.into_iter().map(Into::into).collect(),
        }
    }
}

impl PacketDemuxer for StaticDemuxer {
    fn find_path(&mut self, packet_info: &PacketInfo) -> Option<&NodeRef> {
        for path in &self.packet_paths {
            if (path.predicate)(packet_info) {
                return Some(&path.next);
            }
        }
        None
    }
}

impl_conversion_to_some_packet_handler!(StaticDemuxer, Demuxer);
