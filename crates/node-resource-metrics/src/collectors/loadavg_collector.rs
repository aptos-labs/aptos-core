// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::collectors::common::{MeasureLatency, NAMESPACE};
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::vec;
use sysinfo::{RefreshKind, System, SystemExt};

const LOAD_AVG_METRICS_COUNT: usize = 3;
const LOAD_AVG_SUBSYSTEM: &str = "loadavg";

const LOAD1: &str = "load1";
const LOAD5: &str = "load5";
const LOAD15: &str = "load15";

pub(crate) struct LoadAvgCollector {
    system: System,

    load_one: Desc,
    load_five: Desc,
    load_fifteen: Desc,
}

/// A Collector for exposing host load averages
impl LoadAvgCollector {
    fn new() -> Self {
        let system = System::new_with_specifics(RefreshKind::new());

        let load_one = Opts::new(LOAD1, "1m load average.")
            .namespace(NAMESPACE)
            .subsystem(LOAD_AVG_SUBSYSTEM)
            .describe()
            .unwrap();
        let load_five = Opts::new(LOAD5, "5m load average.")
            .namespace(NAMESPACE)
            .subsystem(LOAD_AVG_SUBSYSTEM)
            .describe()
            .unwrap();
        let load_fifteen = Opts::new(LOAD15, "15m load average.")
            .namespace(NAMESPACE)
            .subsystem(LOAD_AVG_SUBSYSTEM)
            .describe()
            .unwrap();

        Self {
            system,
            load_one,
            load_five,
            load_fifteen,
        }
    }
}

impl Default for LoadAvgCollector {
    fn default() -> Self {
        LoadAvgCollector::new()
    }
}

impl Collector for LoadAvgCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![&self.load_one, &self.load_five, &self.load_fifteen]
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let _measure = MeasureLatency::new("load_avg".into());

        let load_avg = self.system.load_average();

        let load_one = ConstMetric::new_gauge(self.load_one.clone(), load_avg.one, None).unwrap();

        let load_five =
            ConstMetric::new_gauge(self.load_five.clone(), load_avg.five, None).unwrap();

        let load_fifteen =
            ConstMetric::new_gauge(self.load_fifteen.clone(), load_avg.fifteen, None).unwrap();

        let mut mfs = Vec::with_capacity(LOAD_AVG_METRICS_COUNT);
        mfs.extend(load_one.collect());
        mfs.extend(load_fifteen.collect());
        mfs.extend(load_five.collect());

        mfs
    }
}

#[cfg(test)]
mod tests {
    use super::LoadAvgCollector;
    use prometheus::Registry;

    #[test]
    fn test_cpu_collector_register() {
        let collector = LoadAvgCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
