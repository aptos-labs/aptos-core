// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    log::{
        CallFrame, EventStorage, ExecutionAndIOCosts, ExecutionGasEvent, StorageFees, WriteStorage,
        WriteTransient,
    },
    render::Render,
    FrameName, TransactionGasLog,
};
use aptos_gas::{Fee, GasQuantity, GasScalingFactor, InternalGas, InternalGasUnit, Octa};

/// Represents a node in a general tree structure with some text & cost attached to each node.
pub struct Node<U> {
    pub text: String,
    pub cost: GasQuantity<U>,
    pub children: Vec<Node<U>>,
}

/// A type-erased gas log for execution and IO costs.
#[derive(Clone)]
pub struct TypeErasedExecutionAndIoCosts {
    pub gas_scaling_factor: GasScalingFactor,
    pub total: InternalGas,
    pub tree: Node<InternalGasUnit>,
}

/// A type-erased gas log for storage fees.
#[derive(Clone)]
pub struct TypeErasedStorageFees {
    pub total: Fee,
    pub tree: Node<Octa>,
}

/// A gas log with some of the type information erased.
/// "Type-erased" means each item is being tagged with a plain string, rather
/// than some strongly-typed structures.
///
/// This struct serves as an intermediate representation for rendering in textual form.
#[derive(Clone)]
pub struct TypeErasedGasLog {
    pub exec_io: TypeErasedExecutionAndIoCosts,
    pub storage: TypeErasedStorageFees,
}

impl<U> Clone for Node<U> {
    fn clone(&self) -> Self {
        Self {
            text: self.text.clone(),
            cost: self.cost,
            children: self.children.clone(),
        }
    }
}

impl<U> Node<U> {
    pub fn new(name: impl Into<String>, cost: impl Into<GasQuantity<U>>) -> Self {
        Self {
            text: name.into(),
            cost: cost.into(),
            children: vec![],
        }
    }

    pub fn new_with_children(
        name: impl Into<String>,
        cost: impl Into<GasQuantity<U>>,
        children: impl IntoIterator<Item = Self>,
    ) -> Self {
        Self {
            text: name.into(),
            cost: cost.into(),
            children: children.into_iter().collect(),
        }
    }

    pub fn preorder_traversel(&self, mut f: impl FnMut(usize, &str, GasQuantity<U>)) {
        let mut stack = vec![(self, 0)];

        while let Some((node, depth)) = stack.pop() {
            f(depth, &node.text, node.cost);
            stack.extend(node.children.iter().map(|child| (child, depth + 1)).rev());
        }
    }
}

impl ExecutionGasEvent {
    fn to_erased(&self) -> Node<InternalGasUnit> {
        use ExecutionGasEvent::*;

        match self {
            Loc(offset) => Node::new(format!("@{}", offset), 0),
            Bytecode { op, cost } => Node::new(format!("{:?}", op).to_ascii_lowercase(), *cost),
            Call(frame) => frame.to_erased(),
            CallNative {
                module_id,
                fn_name,
                ty_args,
                cost,
            } => Node::new(
                format!(
                    "{}",
                    Render(&(module_id, fn_name.as_ident_str(), ty_args.as_slice()))
                ),
                *cost,
            ),
            LoadResource { addr, ty, cost } => {
                Node::new(format!("load<{}::{}>", Render(addr), ty), *cost)
            },
        }
    }
}

impl CallFrame {
    fn to_erased(&self) -> Node<InternalGasUnit> {
        let name = match &self.name {
            FrameName::Script => "script".to_string(),
            FrameName::Function {
                module_id,
                name,
                ty_args,
            } => {
                format!(
                    "{}",
                    Render(&(module_id, name.as_ident_str(), ty_args.as_slice()))
                )
            },
        };

        let children = self
            .events
            .iter()
            .map(|event| event.to_erased())
            .collect::<Vec<_>>();

        Node::new_with_children(name, 0, children)
    }
}

impl WriteTransient {
    fn to_erased(&self) -> Node<InternalGasUnit> {
        Node::new(
            format!("{}<{}>", Render(&self.op_type), Render(&self.key)),
            self.cost,
        )
    }
}

impl ExecutionAndIOCosts {
    /// Convert the gas log into a type-erased representation.
    pub fn to_erased(&self) -> TypeErasedExecutionAndIoCosts {
        let mut nodes = vec![];

        nodes.push(Node::new("intrinsic", self.intrinsic_cost));
        nodes.push(self.call_graph.to_erased());

        let writes = Node::new_with_children(
            "writes",
            0,
            self.write_set_transient
                .iter()
                .map(|write| write.to_erased()),
        );
        nodes.push(writes);

        TypeErasedExecutionAndIoCosts {
            gas_scaling_factor: self.gas_scaling_factor,
            total: self.total,
            tree: Node::new_with_children("execution & IO (gas unit, full trace)", 0, nodes),
        }
    }
}

impl WriteStorage {
    fn to_erased(&self) -> Node<Octa> {
        Node::new(
            format!("{}<{}>", Render(&self.op_type), Render(&self.key)),
            self.cost,
        )
    }
}

impl EventStorage {
    fn to_erased(&self) -> Node<Octa> {
        Node::new(format!("{}", self.ty), self.cost)
    }
}

impl StorageFees {
    /// Convert the gas log into a type-erased representation.
    #[allow(clippy::vec_init_then_push)]
    pub fn to_erased(&self) -> TypeErasedStorageFees {
        let mut nodes = vec![];

        nodes.push(Node::new("transaction", self.txn_storage));
        nodes.push(Node::new_with_children(
            "writes",
            0,
            self.write_set_storage.iter().map(|write| write.to_erased()),
        ));
        nodes.push(Node::new_with_children(
            "events",
            0,
            self.events.iter().map(|event| event.to_erased()),
        ));

        TypeErasedStorageFees {
            total: self.total,
            tree: Node::new_with_children("storage fees (APT)", 0, nodes),
        }
    }
}

impl TransactionGasLog {
    /// Convert the gas log into a type-erased representation.
    pub fn to_erased(&self) -> TypeErasedGasLog {
        TypeErasedGasLog {
            exec_io: self.exec_io.to_erased(),
            storage: self.storage.to_erased(),
        }
    }
}

impl<U> Node<U> {
    pub fn include_child_costs(&mut self) {
        for child in &mut self.children {
            child.include_child_costs();
            self.cost += child.cost;
        }
    }
}
