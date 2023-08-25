// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Logic for account universes. This is not in the parent module to enforce privacy.

use crate::{account::AccountData, executor::FakeExecutor};
use aptos_proptest_helpers::Index;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use move_ir_compiler::Compiler;
use petgraph::{
    algo::{is_cyclic_directed, toposort},
    Direction, Graph,
};
use proptest::{
    collection::{vec, SizeRange},
    prelude::*,
};
use std::collections::HashMap;

mod module_generator;

#[derive(Debug)]
pub struct Node {
    name: ModuleId,
    self_value: u64,
    account_data: AccountData,
}

#[derive(Debug)]
pub struct DepGraph(Graph<Node, ()>);

impl DepGraph {
    /// Returns a [`Strategy`] that generates a universe of accounts with pre-populated initial
    /// balances.
    pub fn strategy(
        num_accounts: impl Into<SizeRange>,
        num_edges: impl Into<SizeRange>,
        balance_strategy: impl Strategy<Value = u64>,
    ) -> impl Strategy<Value = Self> {
        (
            vec(
                (
                    AccountData::strategy(balance_strategy),
                    any::<u16>(),
                    any::<Identifier>(),
                ),
                num_accounts,
            ),
            vec(any::<(Index, Index)>(), num_edges),
        )
            .prop_map(move |(accounts, indices)| Self::create(accounts, indices))
    }

    fn create(accounts: Vec<(AccountData, u16, Identifier)>, edges: Vec<(Index, Index)>) -> Self {
        let mut graph = Graph::new();
        let indices = accounts
            .into_iter()
            .map(|(account_data, self_value, module_name)| {
                graph.add_node(Node {
                    name: ModuleId::new(*account_data.address(), module_name),
                    self_value: self_value as u64,
                    account_data,
                })
            })
            .collect::<Vec<_>>();
        for (lhs_idx, rhs_idx) in edges {
            let lhs = lhs_idx.get(&indices);
            let rhs = rhs_idx.get(&indices);
            let edge_idx = graph.add_edge(lhs.to_owned(), rhs.to_owned(), ());
            if is_cyclic_directed(&graph) {
                graph.remove_edge(edge_idx);
                let edge_idx = graph.add_edge(rhs.to_owned(), lhs.to_owned(), ());
                if is_cyclic_directed(&graph) {
                    graph.remove_edge(edge_idx);
                }
            }
        }
        Self(graph)
    }

    /// Set up [`DepGraph`] with the initial state generated in this universe.
    pub fn setup(&self, executor: &mut FakeExecutor) {
        for node in self.0.raw_nodes().iter() {
            executor.add_account_data(&node.weight.account_data);
        }
    }

    pub fn execute(&self, executor: &mut FakeExecutor) {
        // Generate a list of modules
        let accounts = toposort(&self.0, None).expect("Dep graph should be acyclic");
        let mut expected_values = HashMap::new();
        let mut modules = vec![];
        for account_idx in accounts.iter().rev() {
            let node = self.0.node_weight(*account_idx).expect("Node should exist");
            let mut result = node.self_value;
            let mut deps = vec![];

            // Calculate the expected result of module entry function
            for successor in self.0.neighbors_directed(*account_idx, Direction::Outgoing) {
                result += expected_values
                    .get(&successor)
                    .expect("Topo sort should already compute the value for successor");
                deps.push(self.0.node_weight(successor).unwrap().name.clone());
            }
            expected_values.insert(*account_idx, result);

            let module_str =
                module_generator::create_module(&node.name, &deps, node.self_value, result);
            let module = Compiler {
                deps: modules.iter().collect(),
            }
            .into_compiled_module(&module_str)
            .expect("Module compilation failed");

            // Publish Module into executor.
            let mut module_bytes = vec![];
            module.serialize(&mut module_bytes).unwrap();

            // TODO: Generate transactions instead of using add_module api.
            executor.add_module(&node.name, module_bytes);

            modules.push(module);
        }
        for account_idx in accounts.iter() {
            let node = self.0.node_weight(*account_idx).expect("Node should exist");

            // TODO: Generate transactions instead of using exec api.
            executor.exec_module(&node.name, "foo", vec![], vec![]);
        }
    }
}
