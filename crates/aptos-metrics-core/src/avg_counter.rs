// Copyright © Aptos Foundation

use prometheus::{register_counter, register_int_counter, Counter, IntCounter};

pub struct AverageCounter {
    sum: Counter,
    count: IntCounter,
}

impl AverageCounter {
    pub fn register(name: &str, desc: &str) -> AverageCounter {
        AverageCounter {
            sum: register_counter!(
                format!("{}_sum", name),
                format!("{}. Sum part of the counter", desc),
            )
            .unwrap(),
            count: register_int_counter!(
                format!("{}_count", name),
                format!("{}. Count part of the counter", desc),
            )
            .unwrap(),
        }
    }

    pub fn observe(&self, value: f64) {
        if value != 0.0 {
            self.sum.inc_by(value);
        }
        self.count.inc();
    }
}

pub struct AverageIntCounter {
    sum: IntCounter,
    count: IntCounter,
}

impl AverageIntCounter {
    pub fn register(name: &str, desc: &str) -> AverageIntCounter {
        AverageIntCounter {
            sum: register_int_counter!(
                format!("{}_sum", name),
                format!("{}. Sum part of the counter", desc),
            )
            .unwrap(),
            count: register_int_counter!(
                format!("{}_count", name),
                format!("{}. Count part of the counter", desc),
            )
            .unwrap(),
        }
    }

    pub fn observe(&self, value: u64) {
        if value != 0 {
            self.sum.inc_by(value);
        }
        self.count.inc();
    }
}
