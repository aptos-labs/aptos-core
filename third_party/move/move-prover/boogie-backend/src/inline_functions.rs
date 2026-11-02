// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Inline Boogie functions whose bodies may refer to each other.
//!
//! Boogie rejects a cycle of inline functions. Behavioral predicates of closure targets form one
//! when a target's spec refers to its own predicates, directly or through other targets or spec
//! functions. Such predicates are deferred to the end of the file, and those on a cycle are
//! emitted without a body: they stay uninterpreted.

use move_model::{code_writer::CodeWriter, emitln};
use petgraph::{algo::tarjan_scc, graphmap::DiGraphMap};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct InlineFunctions {
    /// Bodies of inline functions, emitted in place or deferred, by name.
    bodies: BTreeMap<String, String>,
    /// Name and parameters of the deferred predicates.
    deferred: Vec<(String, String)>,
}

impl InlineFunctions {
    /// Records the body of inline function `name`, already emitted.
    pub fn record_emitted(&mut self, name: String, body: String) {
        self.bodies.insert(name, body);
    }

    /// Defers `function {:inline} <name>(<params>): bool { <body> }`.
    pub fn defer(&mut self, name: String, params: String, body: String) {
        self.bodies.insert(name.clone(), body);
        self.deferred.push((name, params));
    }

    /// Emits the deferred predicates; those on a cycle of inline functions without a body.
    pub fn emit_deferred(&self, writer: &CodeWriter) {
        if self.deferred.is_empty() {
            return;
        }
        let mut graph = DiGraphMap::<&str, ()>::new();
        for (name, body) in &self.bodies {
            graph.add_node(name);
            for callee in identifiers(body).filter(|ident| self.bodies.contains_key(*ident)) {
                graph.add_edge(name, callee, ());
            }
        }
        let on_cycle: BTreeSet<&str> = tarjan_scc(&graph)
            .into_iter()
            .filter(|scc| scc.len() > 1 || graph.contains_edge(scc[0], scc[0]))
            .flatten()
            .collect();
        for (name, params) in &self.deferred {
            if on_cycle.contains(name.as_str()) {
                emitln!(writer, "function {}({}): bool;", name, params);
            } else {
                emitln!(
                    writer,
                    "function {{:inline}} {}({}): bool {{ {} }}",
                    name,
                    params,
                    self.bodies[name]
                );
            }
        }
    }
}

/// The Boogie identifiers in `text`. The names of interest are generated, so a missed one only
/// drops an edge, and Boogie then reports the cycle.
fn identifiers(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || "'~#$^_.?`\\".contains(c)))
        .filter(|token| !token.is_empty())
}
