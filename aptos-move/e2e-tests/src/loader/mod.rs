// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Logic for account universes. This is not in the parent module to enforce privacy.

use crate::{account::AccountData, executor::FakeExecutor};
use aptos_proptest_helpers::Index;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use move_ir_compiler::Compiler;
use petgraph::{algo::toposort, Direction, Graph};
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

/// This module generates a sets of modules that could be used to test the loader.
///
/// To generate the module, a DAG is first generated. The node in this graph represents a module and the edge in this graph represents a dependency
/// between modules. For example if you generate the following DAG:
///
///          M1
///         /  \
///        M2  M3
///
/// This will generate three modules: M1, M2 and M3. Where each module will look like following:
///
/// module 0x3cf3846156a51da7251ccc84b5e4be10c5ab33049b7692d1164fd2c73ef3638b.M2 {
/// public entry foo(): u64 {
///     let a: u64;
/// label b0:
///     a = 49252;                          // randomly generated integer for self node.
///     assert(copy(a) == 49252, 42);       // assert execution result matches expected value. Expected value can be derived from traversing the DAG.
///     return move(a);
/// }
/// }
///
/// module e57d457d2ffacab8f46dea9779f8ad8135c68fd3574eb2902b3da0cfa67cf9d1.M3 {
/// public entry foo(): u64 {
///     let a: u64;
/// label b0:
///     a = 41973;                          // randomly generated integer for self node.
///     assert(copy(a) == 41973, 42);       // assert execution result matches expected value. Expected value can be derived from traversing the DAG.
///     return move(a);
/// }
/// }
///
/// M2 and M3 are leaf nodes so the it should be a no-op function call. M1 will look like following:
///
/// module 0x61da74259dae2c25beb41d11e43c6f5c00cc72d151d8a4f382b02e4e6c420d17.M1 {
/// import 0x3cf3846156a51da7251ccc84b5e4be10c5ab33049b7692d1164fd2c73ef3638b.M2;
/// import e57d457d2ffacab8f46dea9779f8ad8135c68fd3574eb2902b3da0cfa67cf9d1.M3;
/// public entry foo(): u64 {
///     let a: u64;
/// label b0:
///     a = 27856 + M2.foo()+ M3.foo();  // The dependency edge will be converted invokaction into dependent module.
///     assert(copy(a) == 119081, 42);   // The expected value is the sum of self and all its dependent modules.
///     return move(a);
/// }
/// }
///
/// By using this strategy, we can generate a set of modules with complex depenency relationship and assert that the new loader is always
/// linking the call to the right module.
///
/// The next step is to randomly generate module upgrade request to change the structure of DAG to make sure the VM will be able to handle
/// invaldation properly.
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

            if lhs > rhs {
                graph.add_edge(lhs.to_owned(), rhs.to_owned(), ());
            } else if lhs < rhs {
                graph.add_edge(rhs.to_owned(), lhs.to_owned(), ());
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
