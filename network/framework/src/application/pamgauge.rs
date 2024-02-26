// Copyright © Aptos Foundation

use std::collections::HashMap;
use std::sync::Arc;
use aptos_config::network_id::PeerNetworkId;
use aptos_infallible::RwLock;
use aptos_logger::info;
use crate::application::metadata::PeerMetadata;
use crate::application::storage::PeersAndMetadata;

pub struct PeersAndMetadataGauge {
    actual: RwLock<PeersAndMetadataGaugeInner>,
    name: String,
    help: String,
    desc_singleton: prometheus::core::Desc,
}

impl PeersAndMetadataGauge {
    pub fn new(name: String, help: String, peers_and_metadata: Arc<PeersAndMetadata>) -> Self {
        Self {
            actual: RwLock::new(PeersAndMetadataGaugeInner::new(peers_and_metadata)),
            name: name.clone(),
            help: help.clone(),
            desc_singleton: prometheus::core::Desc::new(
                name,
                help,
                vec!["network_id".to_string(), "role_type".to_string(), "direction".to_string()],
                HashMap::new(),
            ).unwrap(),
        }
    }

    pub fn register(name: String, help: String, peers_and_metadata: Arc<PeersAndMetadata>) {
        prometheus::register(Box::new(PeersAndMetadataGauge::new(name, help, peers_and_metadata))).unwrap()
    }
}

impl prometheus::core::Collector for PeersAndMetadataGauge {
    fn desc(&self) -> Vec<&prometheus::core::Desc> {
        vec![&self.desc_singleton]
    }
    fn collect(&self) -> Vec<prometheus::proto::MetricFamily> {
        self.actual.write().collect(self.name.clone(), self.help.clone())
    }
}

struct PeersAndMetadataGaugeInner {
    peers_and_metadata: Arc<PeersAndMetadata>,
    cache_generation: u32,
    cache_peers: Vec<(PeerNetworkId,PeerMetadata)>,
}

impl PeersAndMetadataGaugeInner {
    pub fn new(peers_and_metadata: Arc<PeersAndMetadata>) -> Self {
        Self {
            peers_and_metadata,
            cache_generation: 0,
            cache_peers: Vec::new(),
        }
    }
    pub fn collect(&mut self, name: String, help: String) -> Vec<prometheus::proto::MetricFamily> {
        if let Some((new_peers, new_gen)) = self.peers_and_metadata.get_all_peers_and_metadata_generational(self.cache_generation, true, &[]) {
            self.cache_generation = new_gen;
            self.cache_peers = new_peers;
        }

        let mut counts = HashMap::new();
        for (peer_network_id, peer_metadata) in self.cache_peers.iter() {
            // Before 2024-02 this was: {network_id, peer_id, role_type, direction}
            // But blowing up the metrics with count 1 for every peer_id connected in or out is dumb and wasteful, so skip peer_id.
            let network_id = peer_network_id.network_id();
            let role_type = peer_metadata.connection_metadata.role;
            let direction = peer_metadata.connection_metadata.origin;
            let key = (network_id, direction, role_type);
            counts.entry(key).and_modify(|x| *x = *x + 1).or_insert(1);
        }
        let mut metric_data = Vec::new();
        for (key, count) in counts.iter() {
            let (network_id, direction, role_type) = key;
            info!("metrics: {} {} {}: {}", network_id, role_type, direction, count);
            let labels = vec![
                prom_label_pair("network_id", network_id.as_str()),
                prom_label_pair("role_type", role_type.as_str()),
                prom_label_pair("direction", direction.as_str()),
            ];
            // labels.push(prom_label_pair("network_id", network_id.as_str()));
            // labels.push(prom_label_pair("role_type", role_type.as_str()));
            // labels.push(prom_label_pair("direction", direction.as_str()));
            let mut metric_row = prometheus::proto::Metric::default();
            let mut gauge_value = prometheus::proto::Gauge::default();
            gauge_value.set_value(*count as f64);
            metric_row.set_gauge(gauge_value);
            metric_row.set_label(labels);
            metric_data.push(metric_row);
        }
        let mut metric = prometheus::proto::MetricFamily::default();
        // metric.set_name("aptos_connections".to_string());
        // metric.set_help("Number of current connections and their direction".to_string());
        metric.set_name(name);
        metric.set_help(help);
        metric.set_field_type(prometheus::proto::MetricType::GAUGE);
        metric.set_metric(metric_data);
        return vec![metric];
    }

}

fn prom_label_pair(name: &str, value: &str) -> prometheus::proto::LabelPair {
    let mut out = prometheus::proto::LabelPair::default();
    out.set_name(name.to_string());
    out.set_value(value.to_string());
    out
}
