// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for use of blocks
//! in conditions. Such usage can make code harder to read, so we warn against it.
//!
//! Note that we do allow the use of blocks in conditions if there are inline
//! specifications in them, as this is a common pattern to provide loop invariants.
//!
//! We also only report on the outermost condition with blocks.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, NodeId},
};

/// Expression linter keeping track of traversal state.
#[derive(Default)]
pub struct BlocksInConditions {
    /// Is `None` if we are not traversing a condition.
    state: Option<CondExprState>,
}

/// State of the traversal when a condition expression has been found.
enum CondExprState {
    /// Condition expression with `id` should be examined for:
    ///   the presence of blocks/sequences and the presence of spec blocks.
    Examine { id: NodeId },
    /// Traversing within a condition with `id`. During the traversal:
    ///   `has_spec_block` is true if we have seen a spec block so far.
    ///   `has_any_block` is true if we have seen any block/sequence so far.
    Traversing {
        id: NodeId,
        has_any_block: bool,
        has_spec_block: bool,
    },
}

impl ExpChecker for BlocksInConditions {
    fn get_name(&self) -> String {
        "blocks_in_conditions".to_string()
    }

    fn visit_expr_pre(&mut self, _function: &FunctionEnv, expr: &ExpData) {
        use CondExprState::*;
        use ExpData::{Block, IfElse, Match, Sequence, SpecBlock};
        match self.state {
            None => {
                if let IfElse(_, cond, _, _) | Match(_, cond, _) = expr {
                    self.state = Some(Examine { id: cond.node_id() });
                }
            },
            Some(Examine { id }) if expr.node_id() == id => {
                // We are now starting to traverse a condition.
                let has_any_block = matches!(expr, Block(..) | Sequence(..));
                self.state = Some(Traversing {
                    id,
                    has_any_block,
                    has_spec_block: false, // A spec block cannot appear directly in a condition.
                });
            },
            Some(Traversing {
                id,
                has_any_block,
                has_spec_block,
            }) => {
                let has_any_block = has_any_block || matches!(expr, Block(..) | Sequence(..));
                let has_spec_block = has_spec_block || matches!(expr, SpecBlock(..));
                self.state = Some(Traversing {
                    id,
                    has_any_block,
                    has_spec_block,
                });
            },
            _ => {},
        }
    }

    fn visit_expr_post(&mut self, function: &FunctionEnv, expr: &ExpData) {
        use CondExprState::*;
        match self.state {
            Some(Traversing {
                id,
                has_any_block,
                has_spec_block,
            }) if expr.node_id() == id => {
                // We are done with traversing the condition of interest.
                self.state = None;
                if has_any_block && !has_spec_block {
                    let env = function.env();
                    self.report(
                        env,
                        &env.get_node_loc(id),
                        "Having blocks in conditions make code harder to read. Consider rewriting this code."
                    );
                }
            },
            _ => {},
        }
    }
}
