// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use prometheus::{
    core::{Collector, Desc},
    proto::{Counter, Gauge, LabelPair, Metric, MetricFamily, MetricType},
};

/// Provides a metric with one fixed value that cannot be changed. Users of this
/// package will not have much use for it in regular operations. However, when
/// implementing custom Collectors, it is useful as a throw-away metric that is
/// generated on the fly to send it to Prometheus in the Collect method.
/// Reference: <https://github.com/prometheus/client_golang/blob/main/prometheus/value.go#L106>
#[derive(Debug)]
pub struct ConstMetric {
    desc: Desc,
    metric: Metric,
    metric_type: MetricType,
}

impl ConstMetric {
    pub fn new_counter(
        desc: Desc,
        value: f64,
        label_values: Option<&[String]>,
    ) -> Result<ConstMetric> {
        let label_values = label_values.unwrap_or_default();
        if desc.variable_labels.len() != label_values.len() {
            return Err(anyhow!("label values do not match"));
        }

        let labels = label_pairs(&desc, label_values);

        let mut counter = Counter::default();
        counter.set_value(value);

        let mut metric = Metric::default();
        metric.set_counter(counter);
        metric.set_label(labels.into());

        Ok(ConstMetric {
            desc,
            metric,
            metric_type: MetricType::COUNTER,
        })
    }

    pub fn new_gauge(
        desc: Desc,
        value: f64,
        label_values: Option<&[String]>,
    ) -> Result<ConstMetric> {
        let label_values = label_values.unwrap_or_default();
        if desc.variable_labels.len() != label_values.len() {
            return Err(anyhow!("label values do not match"));
        }

        let labels = label_pairs(&desc, label_values);

        let mut guage = Gauge::default();
        guage.set_value(value);

        let mut metric = Metric::default();
        metric.set_gauge(guage);
        metric.set_label(labels.into());

        Ok(ConstMetric {
            desc,
            metric,
            metric_type: MetricType::GAUGE,
        })
    }
}

impl Collector for ConstMetric {
    fn desc(&self) -> Vec<&Desc> {
        vec![&self.desc]
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let mut met = MetricFamily::default();

        met.set_name(self.desc.fq_name.clone());
        met.set_help(self.desc.help.clone());
        met.set_field_type(self.metric_type);
        met.set_metric(vec![self.metric.clone()].into());

        vec![met]
    }
}

fn label_pairs(desc: &Desc, label_values: &[String]) -> Vec<LabelPair> {
    let labels = Vec::new();

    let total_labels = desc.const_label_pairs.len() + desc.variable_labels.len();
    if total_labels == 0 || label_values.is_empty() {
        return labels;
    }

    let mut labels = desc
        .variable_labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let mut label_pair = LabelPair::default();
            label_pair.set_name(label.clone());
            label_pair.set_value(label_values[i].clone());
            label_pair
        })
        .collect::<Vec<LabelPair>>();

    labels.extend_from_slice(&desc.const_label_pairs);
    labels.sort();
    labels
}

#[cfg(test)]
mod tests {
    use super::ConstMetric;
    use claims::{assert_err, assert_ok};
    use prometheus::{core::Describer, Opts};

    #[test]
    fn test_const_metric_invalid_label_values() {
        let metric_desc = Opts::new("sample_value", "sample test value.")
            .variable_label("test_label")
            .describe()
            .unwrap();

        assert_err!(ConstMetric::new_counter(metric_desc.clone(), 1.0, None));
        assert_err!(ConstMetric::new_gauge(metric_desc.clone(), 1.0, None));

        assert_ok!(ConstMetric::new_counter(
            metric_desc.clone(),
            1.0,
            Some(&["label".into()])
        ));
        assert_ok!(ConstMetric::new_gauge(
            metric_desc,
            1.0,
            Some(&["label".into()])
        ));
    }
}
