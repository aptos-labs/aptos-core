// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregate::AggregatedExecutionGasEvents,
    erased::{Node, TypeErasedExecutionAndIoCosts, TypeErasedGasLog, TypeErasedStorageFees},
};
use aptos_gas::{GasQuantity, InternalGas};
use std::fmt::{self, Write};

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

impl<U> Node<U> {
    fn textualize(
        &self,
        output: &mut impl Write,
        total_cost: GasQuantity<U>,
        include_child_costs: bool,
        fmt_cost: impl Fn(GasQuantity<U>) -> String,
    ) -> fmt::Result {
        let mut alt_tree;
        let tree = if include_child_costs {
            alt_tree = self.clone();
            alt_tree.include_child_costs();
            assert!(
                alt_tree.cost == total_cost,
                "Costs do not add up: expected {}, got {}. There is likely a bug in the profiler.",
                alt_tree.cost,
                total_cost
            );
            &alt_tree
        } else {
            self
        };

        let total_cost = u64::from(total_cost) as f64;

        let mut table = vec![];
        tree.preorder_traversel(|depth, text, cost| {
            let text = format!("{}{}", " ".repeat(depth * 4), text);
            let percentage = if cost.is_zero() {
                "".to_string()
            } else {
                format!("{:.2}%", u64::from(cost) as f64 / total_cost * 100.0)
            };
            let cost = if cost.is_zero() {
                "".to_string()
            } else {
                fmt_cost(cost)
            };

            table.push([text, cost, percentage])
        });

        render_table(output, &table, 4)
    }
}

impl TypeErasedStorageFees {
    pub fn textualize(&self, output: &mut impl Write, include_child_costs: bool) -> fmt::Result {
        self.tree
            .textualize(output, self.total, include_child_costs, |cost| {
                let cost_scaled = format!("{:.8}", (u64::from(cost) as f64 / 1_0000_0000f64));
                crate::misc::strip_trailing_zeros_and_decimal_point(&cost_scaled).to_string()
            })
    }
}

impl TypeErasedExecutionAndIoCosts {
    pub fn textualize(&self, output: &mut impl Write, include_child_costs: bool) -> fmt::Result {
        let scaling_factor = u64::from(self.gas_scaling_factor) as f64;
        self.tree
            .textualize(output, self.total, include_child_costs, |cost| {
                let cost_scaled = format!("{:.8}", (u64::from(cost) as f64 / scaling_factor));
                let cost_scaled =
                    crate::misc::strip_trailing_zeros_and_decimal_point(&cost_scaled).to_string();
                cost_scaled
            })
    }
}

impl TypeErasedGasLog {
    pub fn textualize(&self, output: &mut impl Write, include_child_costs: bool) -> fmt::Result {
        self.exec_io.textualize(output, include_child_costs)?;
        writeln!(output)?;
        self.storage.textualize(output, include_child_costs)?;
        Ok(())
    }
}

impl AggregatedExecutionGasEvents {
    pub fn textualize(&self, output: &mut impl Write) -> fmt::Result {
        let total_cost = u64::from(self.total) as f64;
        let scaling_factor = u64::from(self.gas_scaling_factor) as f64;

        let fmt_item = |name: &str, count: usize, cost: InternalGas| {
            let count = format!("x{}", count);

            let cost_scaled = format!("{:.8}", (u64::from(cost) as f64 / scaling_factor));
            let cost_scaled =
                crate::misc::strip_trailing_zeros_and_decimal_point(&cost_scaled).to_string();

            let percentage = format!("{:.2}%", u64::from(cost) as f64 / total_cost * 100.0);

            [format!("        {}", name), count, cost_scaled, percentage]
        };

        let mut table = vec![];

        table.push([
            "execution & IO (gas unit, aggregated)".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]);

        table.push([
            "    instructions & native calls".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]);
        for (name, count, cost) in &self.ops {
            table.push(fmt_item(name, *count, *cost));
        }

        table.push([
            "    storage reads".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]);
        for (name, count, cost) in &self.storage_reads {
            table.push(fmt_item(name, *count, *cost));
        }

        table.push([
            "    storage writes".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]);
        for (name, count, cost) in &self.storage_writes {
            table.push(fmt_item(name, *count, *cost));
        }

        render_table(output, &table, 4)
    }
}
