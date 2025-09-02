// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    log::{
        CallFrame, Dependency, EventStorage, EventTransient, ExecutionAndIOCosts,
        ExecutionGasEvent, StorageFees, WriteStorage, WriteTransient,
    },
    render::Render,
    FrameName, TransactionGasLog,
};
use aptos_gas_algebra::{Fee, GasScalingFactor, InternalGas};
use std::ops::{Add, AddAssign};

/// Represents a node in a general tree structure where each node is tagged with
/// some text & a numerical value.
#[derive(Clone)]
pub struct Node<N> {
    pub text: String,
    pub val: N,
    pub children: Vec<Node<N>>,
}

/// A type-erased gas log for execution and IO costs.
#[derive(Clone)]
pub struct TypeErasedExecutionAndIoCosts {
    pub gas_scaling_factor: GasScalingFactor,
    pub total: InternalGas,
    pub tree: Node<InternalGas>,
}

#[derive(Clone, Copy)]
pub struct StoragePair {
    pub cost: Fee,
    pub refund: Fee,
}

/// A type-erased gas log for storage fees.
#[derive(Clone)]
pub struct TypeErasedStorageFees {
    pub total: Fee,
    pub tree: Node<StoragePair>,
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

impl StoragePair {
    pub fn zero() -> Self {
        Self {
            cost: 0.into(),
            refund: 0.into(),
        }
    }

    pub fn new(cost: Fee, refund: Fee) -> Self {
        Self { cost, refund }
    }
}

impl Add<Self> for StoragePair {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            cost: self.cost + rhs.cost,
            refund: self.refund + rhs.refund,
        }
    }
}

impl AddAssign<Self> for StoragePair {
    fn add_assign(&mut self, rhs: Self) {
        self.cost += rhs.cost;
        self.refund += rhs.refund;
    }
}

impl<A, B> From<(A, B)> for StoragePair
where
    A: Into<Fee>,
    B: Into<Fee>,
{
    fn from((cost, refund): (A, B)) -> Self {
        Self {
            cost: cost.into(),
            refund: refund.into(),
        }
    }
}

impl<N> Node<N> {
    pub fn new(name: impl Into<String>, data: impl Into<N>) -> Self {
        Self {
            text: name.into(),
            val: data.into(),
            children: vec![],
        }
    }

    pub fn new_with_children(
        name: impl Into<String>,
        data: impl Into<N>,
        children: impl IntoIterator<Item = Self>,
    ) -> Self {
        Self {
            text: name.into(),
            val: data.into(),
            children: children.into_iter().collect(),
        }
    }

    pub fn preorder_traversel(&self, mut f: impl FnMut(usize, &str, &N)) {
        let mut stack = vec![(self, 0)];

        while let Some((node, depth)) = stack.pop() {
            f(depth, &node.text, &node.val);
            stack.extend(node.children.iter().map(|child| (child, depth + 1)).rev());
        }
    }
}

impl ExecutionGasEvent {
    fn to_erased(&self, keep_generic_types: bool) -> Node<InternalGas> {
        use ExecutionGasEvent::*;

        match self {
            Loc(offset) => Node::new(format!("@{}", offset), 0),
            Bytecode { op, cost } => Node::new(format!("{:?}", op).to_ascii_lowercase(), *cost),
            Call(frame) => frame.to_erased(keep_generic_types),
            CallNative {
                module_id,
                fn_name,
                ty_args,
                cost,
            } => Node::new(
                format!(
                    "{}",
                    Render(&(
                        module_id,
                        fn_name.as_ident_str(),
                        if keep_generic_types {
                            ty_args.as_slice()
                        } else {
                            &[]
                        }
                    ))
                ),
                *cost,
            ),
            LoadResource { addr, ty, cost } => Node::new(
                format!("load<{}::{}>", Render(addr), ty.to_canonical_string()),
                *cost,
            ),
            CreateTy { cost } => Node::new("create_ty", *cost),
        }
    }
}

impl CallFrame {
    fn to_erased(&self, keep_generic_types: bool) -> Node<InternalGas> {
        let name = match &self.name {
            FrameName::Script => "script".to_string(),
            FrameName::TransactionBatch => "transaction batch".to_string(),
            FrameName::Function {
                module_id,
                name,
                ty_args,
            } => {
                format!(
                    "{}",
                    Render(&(
                        module_id,
                        name.as_ident_str(),
                        if keep_generic_types {
                            ty_args.as_slice()
                        } else {
                            &[]
                        }
                    ))
                )
            },
        };

        let children = self
            .events
            .iter()
            .map(|event| event.to_erased(keep_generic_types))
            .collect::<Vec<_>>();

        Node::new_with_children(name, 0, children)
    }
}

impl EventTransient {
    fn to_erased(&self) -> Node<InternalGas> {
        Node::new(format!("{}", Render(&self.ty)), self.cost)
    }
}

impl WriteTransient {
    fn to_erased(&self) -> Node<InternalGas> {
        Node::new(
            format!("{}<{}>", Render(&self.op_type), Render(&self.key)),
            self.cost,
        )
    }
}

impl Dependency {
    fn to_erased(&self) -> Node<InternalGas> {
        Node::new(self.render(), self.cost)
    }
}

impl ExecutionAndIOCosts {
    /// Convert the gas log into a type-erased representation.
    pub fn to_erased(&self, keep_generic_types: bool) -> TypeErasedExecutionAndIoCosts {
        let mut nodes = vec![];

        nodes.push(Node::new("intrinsic", self.intrinsic_cost));

        nodes.push(Node::new("keyless", self.keyless_cost));

        if !self.dependencies.is_empty() {
            let deps = Node::new_with_children(
                "dependencies",
                0,
                self.dependencies.iter().map(|dep| dep.to_erased()),
            );
            nodes.push(deps);
        }

        nodes.push(self.call_graph.to_erased(keep_generic_types));

        nodes.push(self.ledger_writes());

        TypeErasedExecutionAndIoCosts {
            gas_scaling_factor: self.gas_scaling_factor,
            total: self.total,
            tree: Node::new_with_children("execution & IO (gas unit, full trace)", 0, nodes),
        }
    }

    fn ledger_writes(&self) -> Node<InternalGas> {
        let transaction = Node::new(
            "transaction",
            self.transaction_transient.unwrap_or_else(|| 0.into()),
        );
        let events = Node::new_with_children(
            "events",
            0,
            self.events_transient.iter().map(|event| event.to_erased()),
        );
        let write_ops = Node::new_with_children(
            "state write ops",
            0,
            self.write_set_transient
                .iter()
                .map(|write| write.to_erased()),
        );

        Node::new_with_children("ledger writes", 0, vec![transaction, events, write_ops])
    }
}

impl WriteStorage {
    fn to_erased(&self) -> Node<StoragePair> {
        Node::new(
            format!("{}<{}>", Render(&self.op_type), Render(&self.key)),
            (self.cost, self.refund),
        )
    }
}

impl EventStorage {
    fn to_erased(&self) -> Node<StoragePair> {
        Node::new(self.ty.to_canonical_string(), (self.cost, Fee::zero()))
    }
}

impl StorageFees {
    /// Convert the gas log into a type-erased representation.
    #[allow(clippy::vec_init_then_push)]
    pub fn to_erased(&self) -> TypeErasedStorageFees {
        let mut nodes = vec![];

        nodes.push(Node::new("transaction", (self.txn_storage, Fee::zero())));
        nodes.push(Node::new_with_children(
            "writes",
            (Fee::zero(), Fee::zero()),
            self.write_set_storage.iter().map(|write| write.to_erased()),
        ));
        nodes.push(Node::new_with_children(
            "events",
            (Fee::zero(), Fee::zero()),
            self.events.iter().map(|event| event.to_erased()),
        ));

        TypeErasedStorageFees {
            total: self.total,
            tree: Node::new_with_children("storage fees (APT)", (Fee::zero(), Fee::zero()), nodes),
        }
    }
}

impl TransactionGasLog {
    /// Convert the gas log into a type-erased representation.
    pub fn to_erased(&self, keep_generic_types: bool) -> TypeErasedGasLog {
        TypeErasedGasLog {
            exec_io: self.exec_io.to_erased(keep_generic_types),
            storage: self.storage.to_erased(),
        }
    }
}

impl<N> Node<N>
where
    N: AddAssign<N> + Copy,
{
    pub fn include_child_costs(&mut self) {
        for child in &mut self.children {
            child.include_child_costs();
            self.val += child.val;
        }
    }
}
