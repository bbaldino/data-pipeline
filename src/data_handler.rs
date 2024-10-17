use anyhow::Result;

use crate::{node::NodeRef, node_visitor::NodeVisitor, stats_producer::StatsProducer};

pub trait DataObserver<T> {
    fn observe(&mut self, data: &T);
    fn get_stats(&self) -> Option<serde_json::Value> {
        None
    }
}

pub trait DataTransformer<T> {
    fn transform(&mut self, data: T) -> Result<T>;
    fn get_stats(&self) -> Option<serde_json::Value> {
        None
    }
}

pub trait DataFilter<T> {
    fn should_forward(&mut self, data: &T) -> bool;
    fn get_stats(&self) -> Option<serde_json::Value> {
        None
    }
}

// impl<T, F> StatsProducer for F where F: FnMut(&T) -> bool {}

impl<T, F> DataFilter<T> for F
where
    F: FnMut(&T) -> bool,
{
    fn should_forward(&mut self, packet_info: &T) -> bool {
        (self)(packet_info)
    }
}

impl<F, T> From<F> for SomeDataHandler<T>
where
    F: FnMut(&T) -> bool + Send + 'static,
{
    fn from(value: F) -> Self {
        SomeDataHandler::Filter(Box::new(value))
    }
}

pub trait DataConsumer<T> {
    fn consume(&mut self, data: T);
    fn get_stats(&self) -> Option<serde_json::Value> {
        None
    }
}

pub trait DataDemuxer<T> {
    fn find_path(&mut self, data: &T) -> Option<&NodeRef<T>>;
    // DataDemuxer has to have its own visitor logic since it handles its own paths
    fn visit(&mut self, visitor: &mut dyn NodeVisitor<T>);
    fn get_stats(&self) -> Option<serde_json::Value> {
        None
    }
}

// Note: Ideally we'd have blanket impls to convert from any of the above traits into
// SomeDatahandler, but unfortunately I don't think that can be done without causing conflicting
// implementation errors.  This macro helps with the conversion at least.
// Example:
// impl_conversion_to_some_data_handler!(MyType, Observer);
#[macro_export]
macro_rules! impl_conversion_to_some_data_handler {
    ($type:ty,$variant:ident) => {
        impl<T> From<$type> for $crate::data_handler::SomeDataHandler<T> {
            fn from(value: $type) -> Self {
                $crate::data_handler::SomeDataHandler::$variant(Box::new(value))
            }
        }
    };
}

pub enum SomeDataHandler<T> {
    Observer(Box<dyn DataObserver<T> + Send>),
    Transformer(Box<dyn DataTransformer<T> + Send>),
    Filter(Box<dyn DataFilter<T> + Send>),
    Consumer(Box<dyn DataConsumer<T> + Send>),
    Demuxer(Box<dyn DataDemuxer<T> + Send>),
}

impl<T> StatsProducer for SomeDataHandler<T> {
    fn get_stats(&self) -> Option<serde_json::Value> {
        match self {
            SomeDataHandler::Observer(ref o) => o.get_stats(),
            SomeDataHandler::Transformer(ref t) => t.get_stats(),
            SomeDataHandler::Filter(ref f) => f.get_stats(),
            SomeDataHandler::Consumer(ref c) => c.get_stats(),
            SomeDataHandler::Demuxer(ref d) => d.get_stats(),
        }
    }
}
