use anyhow::Result;

use crate::{node::NodeRef, stats_producer::StatsProducer};

pub trait PacketObserver<T>: StatsProducer {
    fn observe(&mut self, data: &T);
}

pub trait PacketTransformer<T>: StatsProducer {
    fn transform(&mut self, data: T) -> Result<T>;
}

pub trait PacketFilter<T>: StatsProducer {
    fn should_forward(&mut self, packet_info: &T) -> bool;
}

// impl<T, F> StatsProducer for F where F: FnMut(&T) -> bool {}
//
// impl<T, F> PacketFilter<T> for F
// where
//     F: FnMut(&T) -> bool,
// {
//     fn should_forward(&mut self, packet_info: &T) -> bool {
//         (self)(packet_info)
//     }
// }

pub trait PacketConsumer<T>: StatsProducer {
    fn consume(&mut self, packet_info: T);
}

pub trait PacketDemuxer<T>: StatsProducer {
    fn find_path(&mut self, packet_info: &T) -> Option<&NodeRef<T>>;
    //     // PacketDemuxer has to have its own visitor logic since it handles its own paths
    //     fn visit(&mut self, visitor: &mut dyn NodeVisitor);
}

// Note: Ideally we'd have blanket impls to convert from any of the above traits into
// SomePackethandler, but unfortunately I don't think that can be done without causing conflicting
// implementation errors.  This macro helps with the conversion at least.
#[macro_export]
macro_rules! impl_conversion_to_some_packet_handler {
    ($type:ty,$variant:ident) => {
        impl<T> From<$type> for $crate::packet_handler::SomePacketHandler<T> {
            fn from(value: $type) -> Self {
                $crate::packet_handler::SomePacketHandler::$variant(Box::new(value))
            }
        }
    };
}

pub enum SomePacketHandler<T> {
    Observer(Box<dyn PacketObserver<T> + Send>),
    Transformer(Box<dyn PacketTransformer<T> + Send>),
    Filter(Box<dyn PacketFilter<T> + Send>),
    Consumer(Box<dyn PacketConsumer<T> + Send>),
    Demuxer(Box<dyn PacketDemuxer<T> + Send>),
}

impl<T> StatsProducer for SomePacketHandler<T> {
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
