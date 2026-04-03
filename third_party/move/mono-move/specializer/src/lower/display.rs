// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Display for lowered micro-ops in test baselines.

use super::context::LoweringContext;
use mono_move_core::MicroOp;
use std::fmt;

pub struct MicroOpsFunctionDisplay<'a> {
    pub func_name: &'a str,
    pub ctx: &'a LoweringContext,
    pub ops: &'a [MicroOp],
}

impl fmt::Display for MicroOpsFunctionDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "fun {}() {{", self.func_name)?;
        writeln!(f, "  frame_data_size: {}", self.ctx.frame_data_size)?;
        writeln!(f, "  code:")?;
        for (i, op) in self.ops.iter().enumerate() {
            write!(f, "    {}: {}", i, op)?;
            writeln!(f)?;
        }
        writeln!(f, "}}")
    }
}
