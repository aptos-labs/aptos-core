// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{log::TransactionGasLog, render::Render};
use anyhow::Result;
use aptos_gas_algebra::{Fee, InternalGas};
use handlebars::Handlebars;
use serde_json::{json, Map, Value};
use std::{
    fmt::{self, Write},
    fs,
    path::Path,
};

const TEMPLATE: &str = include_str!("../templates/index.html");
const TRACE_TEMPLATE: &str = include_str!("../templates/trace.html");

fn ensure_dirs_exist(path: impl AsRef<Path>) -> Result<()> {
    if let Err(err) = fs::create_dir_all(&path) {
        match err.kind() {
            std::io::ErrorKind::AlreadyExists => (),
            _ => return Err(err.into()),
        }
    }
    Ok(())
}

fn indent(output: &mut impl Write, count: usize) -> fmt::Result {
    if count == 0 {
        return Ok(());
    }

    write!(output, "{}", " ".repeat(count))
}

fn render_table<R, S>(output: &mut impl Write, table: &[R], spacing: usize) -> fmt::Result
where
    R: AsRef<[S]>,
    S: AsRef<str>,
{
    let n_rows = table.len();
    assert!(n_rows >= 1, "there must be at least 1 row");

    let n_cols = table[0].as_ref().len();
    assert!(n_cols >= 1, "there must be at least 1 col");
    assert!(
        table.iter().skip(1).all(|row| row.as_ref().len() == n_cols),
        "mismatching row widths"
    );

    let text = |row: usize, col: usize| -> &str { table[row].as_ref()[col].as_ref() };

    let col_widths = (0..(n_cols - 1))
        .map(|col| (0..n_rows).map(|row| text(row, col).len()).max().unwrap())
        .collect::<Vec<_>>();

    #[allow(clippy::needless_range_loop)]
    for row in 0..n_rows {
        for col in 0..n_cols {
            if col > 0 {
                indent(output, spacing)?;
            }

            let t = text(row, col);
            write!(output, "{}", t)?;

            if col + 1 < n_cols {
                indent(output, col_widths[col] - t.len())?;
            }
        }
        writeln!(output)?;
    }

    Ok(())
}

impl TransactionGasLog {
    pub fn generate_html_report(&self, path: impl AsRef<Path>, header: String) -> Result<()> {
        let mut data = Map::new();
        data.insert(
            "title".to_string(),
            Value::String(
                if self.num_txns > 1 {
                    format!(
                        "{} - aggregated across {} transactions",
                        header, self.num_txns
                    )
                } else {
                    header
                },
            ),
        );

        // Flamegraphs
        let graph_exec_io = self.exec_io.to_flamegraph("Execution & IO".to_string())?;
        let graph_storage = self.storage.to_flamegraph("Storage".to_string())?;

        data.insert(
            "graph-exec-io".to_string(),
            Value::Bool(graph_exec_io.is_some()),
        );
        data.insert(
            "graph-storage".to_string(),
            Value::Bool(graph_storage.is_some()),
        );

        let scaling_factor = u64::from(self.exec_io.gas_scaling_factor) as f64;

        // Helper to format gas in external units
        let fmt_gas = |gas: InternalGas| -> String {
            let scaled = format!("{:.8}", u64::from(gas) as f64 / scaling_factor);
            crate::misc::strip_trailing_zeros_and_decimal_point(&scaled).to_string()
        };

        // Helper to format fees in APT
        let fmt_apt = |fee: Fee| -> String {
            let scaled = format!("{:.8}", u64::from(fee) as f64 / 1_0000_0000f64);
            crate::misc::strip_trailing_zeros_and_decimal_point(&scaled).to_string()
        };

        data.insert(
            "summary-execution-gas".to_string(),
            Value::String(fmt_gas(self.exec_io.execution_gas)),
        );
        data.insert(
            "summary-io-gas".to_string(),
            Value::String(fmt_gas(self.exec_io.io_gas)),
        );
        data.insert(
            "summary-storage-fee".to_string(),
            Value::String(fmt_apt(self.storage.total)),
        );
        data.insert(
            "summary-storage-refund".to_string(),
            Value::String(fmt_apt(self.storage.total_refund)),
        );

        // Separate totals for execution and IO categories
        let total_execution = u64::from(self.exec_io.execution_gas) as f64;
        let total_io = u64::from(self.exec_io.io_gas) as f64;

        // Helper to calculate percentage against execution gas
        let exec_percentage = |cost: InternalGas| -> String {
            if total_execution == 0.0 {
                "/".to_string()
            } else {
                format!("{:.2}%", u64::from(cost) as f64 / total_execution * 100.0)
            }
        };

        // Helper to calculate percentage against IO gas
        let io_percentage = |cost: InternalGas| -> String {
            if total_io == 0.0 {
                "/".to_string()
            } else {
                format!("{:.2}%", u64::from(cost) as f64 / total_io * 100.0)
            }
        };

        // Intrinsic cost (execution category)
        data.insert(
            "intrinsic".to_string(),
            json!(fmt_gas(self.exec_io.intrinsic_cost)),
        );
        if !self.exec_io.execution_gas.is_zero() {
            data.insert(
                "intrinsic-percentage".to_string(),
                json!(exec_percentage(self.exec_io.intrinsic_cost)),
            );
        }

        // Keyless cost (execution category)
        if !self.exec_io.keyless_cost.is_zero() {
            data.insert(
                "keyless".to_string(),
                json!(fmt_gas(self.exec_io.keyless_cost)),
            );
            data.insert(
                "keyless-percentage".to_string(),
                json!(exec_percentage(self.exec_io.keyless_cost)),
            );
        }

        // SLH-DSA-SHA2-128s cost (execution category)
        if !self.exec_io.slh_dsa_sha2_128s_cost.is_zero() {
            data.insert(
                "slh_dsa_sha2_128s".to_string(),
                json!(fmt_gas(self.exec_io.slh_dsa_sha2_128s_cost)),
            );
            data.insert(
                "slh_dsa_sha2_128s-percentage".to_string(),
                json!(exec_percentage(self.exec_io.slh_dsa_sha2_128s_cost)),
            );
        }

        // Dependencies (execution category - loading modules is CPU work)
        let mut deps = self.exec_io.dependencies.clone();
        deps.sort_by(|lhs, rhs| rhs.cost.cmp(&lhs.cost));
        data.insert(
            "deps".to_string(),
            Value::Array(
                deps.iter()
                    .map(|dep| {
                        json!({
                            "name": dep.render(),
                            "size": u64::from(dep.size),
                            "cost": fmt_gas(dep.cost),
                            "percentage": exec_percentage(dep.cost),
                        })
                    })
                    .collect(),
            ),
        );

        // Execution & IO (aggregated)
        let aggregated = self.exec_io.aggregate_gas_events();

        // Combined total for methods tables (they span both execution and IO)
        let total_combined = total_execution + total_io;
        let combined_percentage = |cost: InternalGas| -> String {
            if total_combined == 0.0 {
                "/".to_string()
            } else {
                format!("{:.2}%", u64::from(cost) as f64 / total_combined * 100.0)
            }
        };

        // Execution category: ops (bytecodes and natives)
        data.insert(
            "ops".to_string(),
            Value::Array(
                aggregated
                    .ops
                    .into_iter()
                    .map(|(name, hits, cost)| {
                        json!({"name": name, "hits": hits, "cost": fmt_gas(cost), "percentage": exec_percentage(cost)})
                    })
                    .collect(),
            ),
        );

        // Methods tables use combined total (they include both execution and IO costs)
        data.insert(
            "methods".to_string(),
            Value::Array(
                aggregated
                    .methods
                    .into_iter()
                    .map(|(name, hits, cost)| {
                        json!({"name": name, "hits": hits, "cost": fmt_gas(cost), "percentage": combined_percentage(cost)})
                    })
                    .collect(),
            ),
        );
        data.insert(
            "methods_self".to_string(),
            Value::Array(
                aggregated
                    .methods_self
                    .into_iter()
                    .map(|(name, hits, cost)| {
                        json!({"name": name, "hits": hits, "cost": fmt_gas(cost), "percentage": combined_percentage(cost)})
                    })
                    .collect(),
            ),
        );

        // IO category: reads, writes, events
        data.insert(
            "reads".to_string(),
            Value::Array(
                aggregated
                    .storage_reads
                    .into_iter()
                    .map(|(name, hits, cost)| {
                        json!({"name": name, "hits": hits, "cost": fmt_gas(cost), "percentage": io_percentage(cost)})
                    })
                    .collect(),
            ),
        );
        data.insert(
            "writes".to_string(),
            Value::Array(
                aggregated
                    .storage_writes
                    .into_iter()
                    .map(|(name, hits, cost)| {
                        json!({"name": name, "hits": hits, "cost": fmt_gas(cost), "percentage": io_percentage(cost)})
                    })
                    .collect(),
            ),
        );
        data.insert(
            "transaction_write".to_string(),
            json!({
                "name": "transaction_write",
                "hits": 1,
                "cost": fmt_gas(aggregated.transaction_write),
                "percentage": io_percentage(aggregated.transaction_write)
            }),
        );
        data.insert(
            "event_writes".to_string(),
            Value::Array(
                aggregated
                    .event_writes
                    .into_iter()
                    .map(|(name, hits, cost)| {
                        json!({"name": name, "hits": hits, "cost": fmt_gas(cost), "percentage": io_percentage(cost)})
                    })
                    .collect(),
            ),
        );

        // Storage fee for the transaction itself
        let total_storage = u64::from(self.storage.total) as f64;
        let total_refund = u64::from(self.storage.total_refund) as f64;

        let storage_percentage = |fee: Fee| -> String {
            if self.storage.total.is_zero() {
                "/".to_string()
            } else {
                format!("{:.2}%", u64::from(fee) as f64 / total_storage * 100.0)
            }
        };

        data.insert(
            "storage-txn".to_string(),
            Value::String(fmt_apt(self.storage.txn_storage)),
        );
        if !self.storage.total.is_zero() {
            data.insert(
                "storage-txn-percentage".to_string(),
                Value::String(storage_percentage(self.storage.txn_storage)),
            );
        }

        // Storage fees & refunds for state changes
        let mut storage_writes = self.storage.write_set_storage.clone();
        storage_writes.sort_by(|lhs, rhs| rhs.cost.cmp(&lhs.cost));
        data.insert(
            "storage-writes".to_string(),
            Value::Array(
                storage_writes
                    .iter()
                    .map(|write| {
                        let (refund_scaled, refund_percentage) = if write.refund.is_zero() {
                            ("/".to_string(), "/".to_string())
                        } else {
                            let percentage = format!(
                                "{:.2}%",
                                u64::from(write.refund) as f64 / total_refund * 100.0
                            );
                            (fmt_apt(write.refund), percentage)
                        };

                        json!({
                            "name": Render(&write.key).to_string(),
                            "cost": fmt_apt(write.cost),
                            "cost-percentage": storage_percentage(write.cost),
                            "refund": refund_scaled,
                            "refund-percentage": refund_percentage
                        })
                    })
                    .collect(),
            ),
        );

        // Storage fees for events
        let mut storage_events = self.storage.events.clone();
        storage_events.sort_by(|lhs, rhs| rhs.cost.cmp(&lhs.cost));
        data.insert(
            "storage-events".to_string(),
            Value::Array(
                storage_events
                    .iter()
                    .map(|event| {
                        json!({
                            "name": event.ty.to_canonical_string(),
                            "cost": fmt_apt(event.cost),
                            "cost-percentage": storage_percentage(event.cost),
                        })
                    })
                    .collect(),
            ),
        );
        if !self.storage.event_discount.is_zero() {
            let discount_msg = format!(
                "*This does not include a discount of {} APT which was applied to reduce the total cost for events.",
                fmt_apt(self.storage.event_discount)
            );
            data.insert(
                "storage-event-discount".to_string(),
                Value::String(discount_msg),
            );
        }

        // Memory usage
        data.insert(
            "peak-memory-usage".to_string(),
            Value::String(format!("{}", self.peak_memory_usage)),
        );

        // Execution trace (shows raw costs without percentages)
        let mut tree = self.exec_io.to_erased(true).tree;
        tree.include_child_costs();

        let mut table = vec![];
        tree.preorder_traversel(|depth, text, &cost| {
            let text_indented = format!("{}{}", " ".repeat(depth * 4), text);
            let cost_str = if cost.is_zero() {
                String::new()
            } else {
                fmt_gas(cost)
            };
            table.push([text_indented, cost_str])
        });

        let mut trace = String::new();
        render_table(&mut trace, &table, 4)?;
        data.insert("trace".to_string(), Value::String(trace));

        // Rendering the html doc
        let mut handlebars = Handlebars::new();
        handlebars.register_template_string("index", TEMPLATE)?;
        let html = handlebars.render("index", &data)?;

        // Writing to disk
        let path_root = path.as_ref();

        ensure_dirs_exist(path_root)?;
        let path_assets = path_root.join("assets");
        ensure_dirs_exist(&path_assets)?;

        if let Some(graph_bytes) = graph_exec_io {
            fs::write(path_assets.join("exec_io.svg"), graph_bytes)?;
        }
        if let Some(graph_bytes) = graph_storage {
            fs::write(path_assets.join("storage.svg"), graph_bytes)?;
        }
        // Write trace to a standalone HTML file for lazy loading via iframe
        // When in iframe: styled with hover effects
        // When opened directly: plain text appearance
        let trace_content: String = trace
            .lines()
            .map(|line| {
                let escaped = line
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                format!("<span class=\"line\">{}</span>", escaped)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let mut trace_data = Map::new();
        trace_data.insert("trace-content".to_string(), json!(trace_content));
        let trace_html = handlebars.render_template(TRACE_TEMPLATE, &trace_data)?;
        fs::write(path_assets.join("trace.html"), trace_html)?;
        fs::write(path_root.join("index.html"), html)?;

        Ok(())
    }
}
