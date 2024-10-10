pub trait StatsProducer {
    fn get_stats(&self) -> Option<serde_json::Value> {
        None
    }
}
