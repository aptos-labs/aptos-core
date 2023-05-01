// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::log::{
    CallFrame, ExecutionGasEvent, FrameName, StorageFees, TransactionGasLog, WriteOpType,
};
use aptos_types::{
    access_path::Path,
    state_store::{
        state_key::{StateKey, StateKeyInner},
        table::TableHandle,
    },
};
use inferno::flamegraph::TextTruncateDirection;
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::InternalGas,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
};
use regex::Captures;
use std::{
    fmt::{self, Display},
    ops::Deref,
};

/// Wrapper to help render the underlying data in formats desirable by the flamegraph.
struct Render<'a, T>(&'a T);

impl<'a> Display for Render<'a, AccountAddress> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let addr_short = self.0.short_str_lossless();
        write!(f, "0x")?;
        if addr_short.len() > 4 {
            write!(f, "{}..", &addr_short[..4])
        } else {
            write!(f, "{}", addr_short)
        }
    }
}

impl<'a> Display for Render<'a, ModuleId> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", Render(self.0.address()), self.0.name())
    }
}

impl<'a> Display for Render<'a, (&'a ModuleId, &'a IdentStr, &'a [TypeTag])> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", Render(self.0 .0), self.0 .1)?;
        if !self.0 .2.is_empty() {
            write!(
                f,
                "<{}>",
                self.0
                     .2
                    .iter()
                    .map(|ty| format!("{}", ty))
                    .collect::<Vec<_>>()
                    .join(",")
            )?;
        }
        Ok(())
    }
}

impl Display for FrameName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Script => write!(f, "<script>"),
            Self::Function {
                module_id,
                name: fn_name,
                ty_args,
            } => write!(
                f,
                "{}",
                Render(&(module_id, fn_name.as_ident_str(), ty_args.as_slice())),
            ),
        }
    }
}

impl<'a> Display for Render<'a, Path> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Path::Code(module_id) => write!(f, "{}", Render(module_id)),
            Path::Resource(struct_ty) => write!(f, "{}", struct_ty),
            Path::ResourceGroup(struct_ty) => write!(f, "{}", struct_ty),
        }
    }
}

impl<'a> Display for Render<'a, TableHandle> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Render(&self.0 .0))
    }
}

struct TableKey<'a> {
    bytes: &'a [u8],
}

impl<'a> Display for TableKey<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.bytes.len() > 2);
        write!(f, "0x{:02x}{:02x}..", self.bytes[0], self.bytes[1])
    }
}

impl<'a> Display for Render<'a, StateKey> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use StateKeyInner::*;

        match self.0.deref() {
            AccessPath(ap) => {
                write!(f, "{}::{}", Render(&ap.address), Render(&ap.get_path()))
            },
            TableItem { handle, key } => {
                write!(f, "table_item<{},{}>", Render(handle), TableKey {
                    bytes: key
                },)
            },
            Raw(..) => panic!("not supported"),
        }
    }
}

impl<'a> Display for Render<'a, WriteOpType> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use WriteOpType::*;

        write!(f, "{}", match self.0 {
            Creation => "create",
            Modification => "modify",
            Deletion => "delete",
        })
    }
}

fn strip_trailing_zeros_and_decimal_point(mut s: &str) -> &str {
    while let Some(stripped) = s.strip_suffix('0') {
        s = stripped
    }
    match s.strip_suffix('.') {
        Some(stripped) => stripped,
        None => s,
    }
}

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
            lines.push(format!("events;{}", event.ty), event.cost)
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

impl TransactionGasLog {
    /// Convert the execution gas log into folded stack lines, which can
    /// then be used to generate a flamegraph.
    fn to_folded_stack_lines(&self) -> Vec<String> {
        let mut lines = LineBuffer::new();

        lines.push("intrinsic", self.intrinsic_cost);

        let mut path = vec![];

        struct Rec<'a> {
            lines: &'a mut LineBuffer,
            path: &'a mut Vec<String>,
        }

        impl<'a> Rec<'a> {
            fn visit(&mut self, frame: &CallFrame) {
                self.path.push(format!("{}", frame.name));

                let mut frame_cost = InternalGas::new(0);

                for event in &frame.events {
                    use ExecutionGasEvent::*;

                    match event {
                        Loc(_) => (),
                        Bytecode { cost, .. } => frame_cost += *cost,
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
                            format!("{};load<{}::{}>", self.path(), Render(addr), ty),
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

        for item in &self.write_set_transient {
            lines.push(
                format!("write_set;{}<{}>", Render(&item.op_type), Render(&item.key)),
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
                strip_trailing_zeros_and_decimal_point(&format!("{:.8}", count_scaled))
            )
        });

        Ok(Some(graph_content.as_bytes().to_vec()))
    }
}
