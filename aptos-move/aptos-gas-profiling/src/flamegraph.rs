// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    log::{CallFrame, ExecutionAndIOCosts, ExecutionGasEvent, StorageFees},
    render::Render,
};
use inferno::flamegraph::TextTruncateDirection;
use move_core_types::gas_algebra::InternalGas;
use regex::Captures;

#[derive(Debug)]
struct LineBuffer(Vec<String>);

impl LineBuffer {
    fn new() -> Self {
        Self(vec![])
    }

    fn push(&mut self, item: impl AsRef<str>, count: impl Into<u64>) {
        let count: u64 = count.into();

        if count > 0 {
            self.0.push(format!("{} {}", item.as_ref(), count));
        }
    }

    fn into_inner(self) -> Vec<String> {
        self.0
    }
}

impl StorageFees {
    /// Convert the storage fee log into folded stack lines, which can
    /// then be used to generate a flamegraph.
    fn to_folded_stack_lines(&self) -> Vec<String> {
        let mut lines = LineBuffer::new();

        lines.push("transaction", self.txn_storage);

        for item in &self.write_set_storage {
            lines.push(
                format!("write_set;{}<{}>", Render(&item.op_type), Render(&item.key)),
                item.cost,
            )
        }

        for event in &self.events {
            // TODO: Handle discounts.
            lines.push(
                format!("events;{}", event.ty.to_canonical_string()),
                event.cost,
            )
        }

        lines.into_inner()
    }

    /// Tries to generate a flamegraph from the execution log.
    /// None will be returned if the log is empty.
    pub fn to_flamegraph(&self, title: String) -> anyhow::Result<Option<Vec<u8>>> {
        let lines = self.to_folded_stack_lines();

        if lines.is_empty() {
            return Ok(None);
        }

        let mut options = inferno::flamegraph::Options::default();
        options.flame_chart = true;
        options.text_truncate_direction = TextTruncateDirection::Right;
        options.color_diffusion = true;
        options.title = title;

        let mut graph_content = vec![];
        inferno::flamegraph::from_lines(
            &mut options,
            lines.iter().rev().map(|s| s.as_str()),
            &mut graph_content,
        )?;
        let graph_content = String::from_utf8_lossy(&graph_content);

        // Inferno does not allow us to customize some of the text in the resulting graph,
        // so we have to do it through regex replacement.
        let re = regex::Regex::new("([1-9][0-9]*(,[0-9]+)*) samples")
            .expect("should be able to build regex successfully");
        let graph_content = re.replace_all(&graph_content, |caps: &Captures| {
            let count: u64 = caps[1]
                .replace(',', "")
                .parse()
                .expect("should be able parse count as u64");

            format!("{} Octa", count)
        });

        Ok(Some(graph_content.as_bytes().to_vec()))
    }
}

impl ExecutionAndIOCosts {
    /// Convert the execution gas log into folded stack lines, which can
    /// then be used to generate a flamegraph.
    fn to_folded_stack_lines(&self) -> Vec<String> {
        let mut lines = LineBuffer::new();

        lines.push("intrinsic", self.intrinsic_cost);

        lines.push("keyless", self.keyless_cost);

        let mut path = vec![];

        struct Rec<'a> {
            lines: &'a mut LineBuffer,
            path: &'a mut Vec<String>,
        }

        for dep in &self.dependencies {
            lines.push(
                format!(
                    "dependencies;{}{}",
                    Render(&dep.id),
                    if dep.is_new { "(new)" } else { "" }
                ),
                dep.cost,
            )
        }

        impl Rec<'_> {
            fn visit(&mut self, frame: &CallFrame) {
                self.path.push(format!("{}", frame.name));

                let mut frame_cost = InternalGas::new(0);

                for event in &frame.events {
                    use ExecutionGasEvent::*;

                    match event {
                        Loc(_) => (),
                        Bytecode { cost, .. } | CreateTy { cost } => frame_cost += *cost,
                        Call(inner_frame) => self.visit(inner_frame),
                        CallNative {
                            module_id: module,
                            fn_name,
                            ty_args,
                            cost,
                        } => self.lines.push(
                            format!(
                                "{};{}",
                                self.path(),
                                Render(&(module, fn_name.as_ident_str(), ty_args.as_slice())),
                            ),
                            *cost,
                        ),
                        LoadResource { addr, ty, cost } => self.lines.push(
                            format!(
                                "{};load<{}::{}>",
                                self.path(),
                                Render(addr),
                                ty.to_canonical_string()
                            ),
                            *cost,
                        ),
                    }
                }

                self.lines.push(&self.path(), frame_cost);
                self.path.pop();
            }

            fn path(&self) -> String {
                self.path.join(";")
            }
        }

        Rec {
            lines: &mut lines,
            path: &mut path,
        }
        .visit(&self.call_graph);

        if let Some(cost) = &self.transaction_transient {
            lines.push("ledger_writes;transaction", *cost)
        }
        for item in &self.events_transient {
            lines.push(
                format!("ledger_writes;events;{}", Render(&item.ty)),
                item.cost,
            )
        }
        for item in &self.write_set_transient {
            lines.push(
                format!(
                    "ledger_writes;state_write_ops;{}<{}>",
                    Render(&item.op_type),
                    Render(&item.key)
                ),
                item.cost,
            )
        }

        lines.into_inner()
    }

    /// Tries to generate a flamegraph from the execution log.
    /// None will be returned if the log is empty.
    pub fn to_flamegraph(&self, title: String) -> anyhow::Result<Option<Vec<u8>>> {
        let lines = self.to_folded_stack_lines();

        if lines.is_empty() {
            return Ok(None);
        }

        let mut options = inferno::flamegraph::Options::default();
        options.flame_chart = true;
        options.text_truncate_direction = TextTruncateDirection::Right;
        options.color_diffusion = true;
        options.title = title;

        let mut graph_content = vec![];
        inferno::flamegraph::from_lines(
            &mut options,
            lines.iter().rev().map(|s| s.as_str()),
            &mut graph_content,
        )?;
        let graph_content = String::from_utf8_lossy(&graph_content);

        // Inferno does not allow us to customize some of the text in the resulting graph,
        // so we have to do it through regex replacement.
        let re = regex::Regex::new("([1-9][0-9]*(,[0-9]+)*) samples")
            .expect("should be able to build regex successfully");
        let graph_content = re.replace_all(&graph_content, |caps: &Captures| {
            let count: u64 = caps[1]
                .replace(',', "")
                .parse()
                .expect("should be able parse count as u64");

            let count_scaled = count as f64 / u64::from(self.gas_scaling_factor) as f64;

            format!(
                "{} gas units",
                crate::misc::strip_trailing_zeros_and_decimal_point(&format!(
                    "{:.8}",
                    count_scaled
                ))
            )
        });

        Ok(Some(graph_content.as_bytes().to_vec()))
    }
}
