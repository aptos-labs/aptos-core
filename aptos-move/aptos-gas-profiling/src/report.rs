// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
        data.insert("title".to_string(), Value::String(header));

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

        // Intrinsic cost
        let scaling_factor = u64::from(self.exec_io.gas_scaling_factor) as f64;
        let total_exec_io = u64::from(self.exec_io.total) as f64;

        let cost_scaled = format!(
            "{:.8}",
            (u64::from(self.exec_io.intrinsic_cost) as f64 / scaling_factor)
        );
        let cost_scaled = crate::misc::strip_trailing_zeros_and_decimal_point(&cost_scaled);
        let percentage = format!(
            "{:.2}%",
            u64::from(self.exec_io.intrinsic_cost) as f64 / total_exec_io * 100.0
        );
        data.insert("intrinsic".to_string(), json!(cost_scaled));
        if !self.exec_io.total.is_zero() {
            data.insert("intrinsic-percentage".to_string(), json!(percentage));
        }

        // Keyless cost
        if !self.exec_io.keyless_cost.is_zero() {
            let cost_scaled = format!(
                "{:.8}",
                (u64::from(self.exec_io.keyless_cost) as f64 / scaling_factor)
            );
            let percentage = format!(
                "{:.2}%",
                u64::from(self.exec_io.keyless_cost) as f64 / total_exec_io * 100.0
            );
            data.insert("keyless".to_string(), json!(cost_scaled));
            data.insert("keyless-percentage".to_string(), json!(percentage));
        }

        let mut deps = self.exec_io.dependencies.clone();
        deps.sort_by(|lhs, rhs| rhs.cost.cmp(&lhs.cost));
        data.insert(
            "deps".to_string(),
            Value::Array(
                deps.iter()
                    .map(|dep| {
                        let name = format!(
                            "{}{}",
                            Render(&dep.id),
                            if dep.is_new { " (new)" } else { "" }
                        );
                        let cost_scaled =
                            format!("{:.8}", (u64::from(dep.cost) as f64 / scaling_factor));
                        let cost_scaled =
                            crate::misc::strip_trailing_zeros_and_decimal_point(&cost_scaled);
                        let percentage =
                            format!("{:.2}%", u64::from(dep.cost) as f64 / total_exec_io * 100.0);

                        json!({
                            "name": name,
                            "size": u64::from(dep.size),
                            "cost": cost_scaled,
                            "percentage": percentage,
                        })
                    })
                    .collect(),
            ),
        );

        // Execution & IO (aggregated)
        let aggregated: crate::aggregate::AggregatedExecutionGasEvents =
            self.exec_io.aggregate_gas_events();
        let convert_op = |(op, hits, cost): (String, usize, InternalGas)| {
            let cost_scaled = format!("{:.8}", (u64::from(cost) as f64 / scaling_factor));
            let cost_scaled = crate::misc::strip_trailing_zeros_and_decimal_point(&cost_scaled);

            let percentage = format!("{:.2}%", u64::from(cost) as f64 / total_exec_io * 100.0);

            json!({
                "name": op,
                "hits": hits,
                "cost": cost_scaled,
                "percentage": percentage,
            })
        };
        data.insert(
            "ops".to_string(),
            Value::Array(aggregated.ops.into_iter().map(convert_op).collect()),
        );
        data.insert(
            "reads".to_string(),
            Value::Array(
                aggregated
                    .storage_reads
                    .into_iter()
                    .map(convert_op)
                    .collect(),
            ),
        );
        data.insert(
            "writes".to_string(),
            Value::Array(
                aggregated
                    .storage_writes
                    .into_iter()
                    .map(convert_op)
                    .collect(),
            ),
        );
        data.insert(
            "transaction_write".to_string(),
            convert_op((
                "transaction_write".to_string(),
                1,
                aggregated.transaction_write,
            )),
        );
        data.insert(
            "event_writes".to_string(),
            Value::Array(
                aggregated
                    .event_writes
                    .into_iter()
                    .map(convert_op)
                    .collect(),
            ),
        );

        // Storage fee for the transaction itself
        let total_storage = u64::from(self.storage.total) as f64;
        let total_refund = u64::from(self.storage.total_refund) as f64;

        let fmt_storage_fee = |fee: Fee| -> String {
            let scaled = format!("{:.8}", (u64::from(fee) as f64 / 1_0000_0000f64));
            crate::misc::strip_trailing_zeros_and_decimal_point(&scaled).to_string()
        };
        let fmt_storage_fee_percentage = |fee: Fee| -> String {
            if self.storage.total.is_zero() {
                "/".to_string()
            } else {
                format!("{:.2}%", u64::from(fee) as f64 / total_storage * 100.0)
            }
        };

        data.insert(
            "storage-txn".to_string(),
            Value::String(fmt_storage_fee(self.storage.txn_storage)),
        );
        if !self.storage.total.is_zero() {
            data.insert(
                "storage-txn-percentage".to_string(),
                Value::String(fmt_storage_fee_percentage(self.storage.txn_storage)),
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
                            let scaled =
                                format!("{:.8}", (u64::from(write.refund) as f64 / 1_0000_0000f64));
                            let scaled =
                                crate::misc::strip_trailing_zeros_and_decimal_point(&scaled);

                            let percentage = format!(
                                "{:.2}%",
                                u64::from(write.refund) as f64 / total_refund * 100.0
                            );
                            (scaled.to_string(), percentage)
                        };

                        json!({
                            "name":  format!("{}", Render(&write.key)),
                            "cost": fmt_storage_fee(write.cost),
                            "cost-percentage": fmt_storage_fee_percentage(write.cost),
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
                            "name":  format!("{}", event.ty.to_canonical_string()),
                            "cost": fmt_storage_fee(event.cost),
                            "cost-percentage": fmt_storage_fee_percentage(event.cost),
                        })
                    })
                    .collect(),
            ),
        );
        if !self.storage.event_discount.is_zero() {
            let discount_msg = format!(
                "*This does not include a discount of {} APT which was applied to reduce the total cost for events.",
                fmt_storage_fee(self.storage.event_discount)
            );
            data.insert(
                "storage-event-discount".to_string(),
                Value::String(discount_msg),
            );
        }

        // Execution trace
        let mut tree = self.exec_io.to_erased().tree;
        tree.include_child_costs();

        let mut table = vec![];
        tree.preorder_traversel(|depth, text, &cost| {
            let text_indented = format!("{}{}", " ".repeat(depth * 4), text);

            if cost.is_zero() {
                table.push([text_indented, "".to_string(), "".to_string()])
            } else {
                let cost_scaled = format!("{:.8}", (u64::from(cost) as f64 / scaling_factor));
                let cost_scaled = crate::misc::strip_trailing_zeros_and_decimal_point(&cost_scaled);

                let percentage = format!("{:.2}%", u64::from(cost) as f64 / total_exec_io * 100.0);

                table.push([text_indented, cost_scaled.to_string(), percentage])
            }
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
        fs::write(path_root.join("index.html"), html)?;

        Ok(())
    }
}
