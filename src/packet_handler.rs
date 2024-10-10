use anyhow::Result;

use crate::{node::NodeRef, packet_info::PacketInfo, stats_producer::StatsProducer};

pub trait PacketObserver: StatsProducer {
    fn observe(&mut self, data: &PacketInfo);
}

pub trait PacketTransformer: StatsProducer {
    fn transform(&mut self, data: PacketInfo) -> Result<PacketInfo>;
}

pub trait PacketFilter: StatsProducer {
    fn should_forward(&mut self, packet_info: &PacketInfo) -> bool;
}

impl<F> StatsProducer for F where F: FnMut(&PacketInfo) -> bool {}

impl<F> PacketFilter for F
where
    F: FnMut(&PacketInfo) -> bool,
{
    fn should_forward(&mut self, packet_info: &PacketInfo) -> bool {
        (self)(packet_info)
    }
}

pub trait PacketConsumer: StatsProducer {
    fn consume(&mut self, packet_info: PacketInfo);
}

pub trait PacketDemuxer: StatsProducer {
    fn find_path(&mut self, packet_info: &PacketInfo) -> Option<&NodeRef>;
    //     // PacketDemuxer has to have its own visitor logic since it handles its own paths
    //     fn visit(&mut self, visitor: &mut dyn NodeVisitor);
}

// Note: Ideally we'd have blanket impls to convert from any of the above traits into
// SomePackethandler, but unfortunately I don't think that can be done without causing conflicting
// implementation errors.  This macro helps with the conversion at least.
#[macro_export]
macro_rules! impl_conversion_to_some_packet_handler {
    ($type:ty,$variant:ident) => {
        impl From<$type> for $crate::packet_handler::SomePacketHandler {
            fn from(value: $type) -> Self {
                $crate::packet_handler::SomePacketHandler::$variant(Box::new(value))
            }
        }
    };
}

pub enum SomePacketHandler {
    Observer(Box<dyn PacketObserver + Send>),
    Transformer(Box<dyn PacketTransformer + Send>),
    Filter(Box<dyn PacketFilter + Send>),
    Consumer(Box<dyn PacketConsumer + Send>),
    Demuxer(Box<dyn PacketDemuxer + Send>),
}

impl StatsProducer for SomePacketHandler {
    fn get_stats(&self) -> Option<serde_json::Value> {
        match self {
            SomePacketHandler::Observer(ref o) => o.get_stats(),
            SomePacketHandler::Transformer(ref t) => t.get_stats(),
            SomePacketHandler::Filter(ref f) => f.get_stats(),
            SomePacketHandler::Consumer(ref c) => c.get_stats(),
            SomePacketHandler::Demuxer(ref d) => d.get_stats(),
        }
    }
}
