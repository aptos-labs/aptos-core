// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Manages the various information during generation

use crate::{
    config::Config,
    names::{Identifier, IdentifierPool, IdentifierType as IDType, Scope},
    types::{Type, TypePool},
};
use arbitrary::Unstructured;
use log::trace;

#[derive(Debug, Default)]
pub struct Env {
    pub id_pool: IdentifierPool,
    pub type_pool: TypePool,

    /// For controlling the depth of the generated expressions/types
    max_expr_depth: usize,
    max_expr_depth_history: Vec<usize>,

    expr_depth: usize,
    expr_depth_history: Vec<usize>,

    type_depth: usize,
    type_depth_history: Vec<usize>,
}

impl Env {
    /// Create a new environment with the given configuration
    pub fn new(config: &Config) -> Self {
        Self {
            id_pool: IdentifierPool::new(),
            type_pool: TypePool::new(),

            max_expr_depth: config.max_expr_depth,
            max_expr_depth_history: vec![],
            expr_depth: 0,
            expr_depth_history: vec![],
            type_depth: 0,
            type_depth_history: vec![],
        }
    }

    /// Return a list of identifiers fileterd by the given type and scope
    /// `typ` should be the desired Move type
    /// `ident_type` should be the desired identifier type (e.g. var, func)
    /// `scope` should be the desired scope
    pub fn get_identifiers(
        &self,
        typ: Option<&Type>,
        ident_type: Option<IDType>,
        scope: Option<&Scope>,
    ) -> Vec<Identifier> {
        // Filter based on the IDType
        let all_ident = match ident_type {
            Some(t) => self.id_pool.get_identifiers_of_ident_type(t),
            None => self.id_pool.get_all_identifiers(),
        };

        // Filter based on Scope
        let ident_in_scope = match scope {
            Some(s) => self.id_pool.filter_identifier_in_scope(&all_ident, s),
            None => all_ident,
        };

        // Filter based on Type
        let type_matched = match typ {
            Some(t) => self
                .type_pool
                .filter_identifier_with_type(t, ident_in_scope),
            None => ident_in_scope,
        };

        // Filter out the identifiers that do not have a type
        // i.e. the one just declared but the RHS of assign is not finished yet
        type_matched
            .into_iter()
            .filter(|id| self.type_pool.get_type(id).is_some())
            .collect()
    }

    /// Return whether the current expression depth has reached the limit
    pub fn reached_expr_depth_limit(&self) -> bool {
        self.expr_depth >= self.max_expr_depth
    }

    /// Return whether the current expression depth will reach the limit
    /// with `inc` more layers
    pub fn will_reached_expr_depth_limit(&self, inc: usize) -> bool {
        self.expr_depth + inc >= self.max_expr_depth
    }

    /// Return the current expression depth
    pub fn curr_expr_depth(&self) -> usize {
        self.expr_depth
    }

    /// Set a temporary maximum expression depth.
    /// Old value will be recorded and can be restored by `reset_max_expr_depth`
    pub fn set_max_expr_depth(&mut self, max_expr_depth: usize) {
        self.max_expr_depth_history.push(self.max_expr_depth);
        self.max_expr_depth = max_expr_depth;
    }

    /// Restore the maximum expression depth to the previous value.
    /// Should always be called with `set_max_expr_depth` in pair
    pub fn reset_max_expr_depth(&mut self) {
        self.max_expr_depth = self.max_expr_depth_history.pop().unwrap();
    }

    /// Randomly choose a number of depth to increase the expression depth.
    /// This allows us to end early in some cases.
    pub fn increase_expr_depth(&mut self, u: &mut Unstructured) {
        let inc = u.choose(&[1, 2, 3]).unwrap();
        self.expr_depth += *inc;
        self.expr_depth_history.push(*inc);
        trace!("Increment expr depth by {} to: {}", *inc, self.expr_depth,);
    }

    /// Decrease the expression depth by the last increased amount.
    /// This should be called after `increase_expr_depth` and
    /// they should always be in pairs.
    pub fn decrease_expr_depth(&mut self) {
        let dec = self.expr_depth_history.pop().unwrap();
        self.expr_depth -= dec;
        trace!("Decrement expr depth to: {}", self.expr_depth);
    }

    /// Randomly choose a number of depth to increase the type depth.
    /// This allows us to end early in some cases.
    pub fn increase_type_depth(&mut self, u: &mut Unstructured) {
        let inc = u.choose(&[1, 2, 3]).unwrap();
        self.type_depth += *inc;
        self.type_depth_history.push(*inc);
        trace!("Increment type depth by {} to: {}", *inc, self.type_depth,);
    }

    /// Decrease the type depth by the last increased amount.
    /// This should be called after `increase_type_depth` and
    /// they should always be in pairs.
    pub fn decrease_type_depth(&mut self) {
        let dec = self.type_depth_history.pop().unwrap();
        self.type_depth -= dec;
        trace!("Decrement type depth to: {}", self.type_depth);
    }
}
