// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use crate::algebra::{AbstractValueSize, AbstractValueSizePerArg};
use crate::gas_meter::{FromOnChainGasSchedule, InitialGasSchedule, ToOnChainGasSchedule};
use move_core_types::{account_address::AccountAddress, gas_algebra::NumArgs};
use move_vm_types::views::{ValueView, ValueVisitor};

crate::params::define_gas_parameters!(
    AbstractValueSizeGasParameters,
    "misc.abs_val",
    [
        // abstract value size
        [u8: AbstractValueSize, "u8", 1],
        [u64: AbstractValueSize, "u64", 8],
        [u128: AbstractValueSize, "u128", 16],
        [bool: AbstractValueSize, "bool", 1],
        [address: AbstractValueSize, "address", 32],
        [struct_: AbstractValueSize, "struct", 8],
        [vector: AbstractValueSize, "vector", 16],
        [reference: AbstractValueSize, "reference", 16],
        [per_u8_packed: AbstractValueSizePerArg, "per_u8_packed", 1],
        [per_u64_packed: AbstractValueSizePerArg, "per_u64_packed", 8],
        [
            per_u128_packed: AbstractValueSizePerArg,
            "per_u128_packed",
            16
        ],
        [
            per_bool_packed: AbstractValueSizePerArg,
            "per_bool_packed",
            1
        ],
        [
            per_address_packed: AbstractValueSizePerArg,
            "per_address_packed",
            32
        ],
    ]
);

struct DerefVisitor<V> {
    inner: V,
    offset: usize,
}

impl<V> DerefVisitor<V>
where
    V: ValueVisitor,
{
    pub fn new(visitor: V) -> Self {
        Self {
            inner: visitor,
            offset: 0,
        }
    }

    pub fn into_inner(self) -> V {
        self.inner
    }
}

macro_rules! deref_visitor_delegate_simple {
    ($([$fn: ident, $ty: ty $(,)?]),+ $(,)?) => {
        $(
            #[inline]
            fn $fn(&mut self, depth: usize, val: $ty) {
                self.inner.$fn(depth - self.offset, val);
            }
        )*
    };
}

impl<V> ValueVisitor for DerefVisitor<V>
where
    V: ValueVisitor,
{
    deref_visitor_delegate_simple!(
        [visit_u8, u8],
        [visit_u64, u64],
        [visit_u128, u128],
        [visit_bool, bool],
        [visit_address, AccountAddress],
        [visit_vec_u8, &[u8]],
        [visit_vec_u64, &[u64]],
        [visit_vec_u128, &[u128]],
        [visit_vec_bool, &[bool]],
        [visit_vec_address, &[AccountAddress]],
    );

    #[inline]
    fn visit_struct(&mut self, depth: usize, len: usize) -> bool {
        self.inner.visit_struct(depth - self.offset, len)
    }

    #[inline]
    fn visit_vec(&mut self, depth: usize, len: usize) -> bool {
        self.inner.visit_vec(depth - self.offset, len)
    }

    #[inline]
    fn visit_ref(&mut self, depth: usize, _is_global: bool) -> bool {
        assert_eq!(depth, 0, "There shouldn't be inner refs");
        self.offset = 1;
        true
    }
}

struct AbstractValueSizeVisitor<'a> {
    params: &'a AbstractValueSizeGasParameters,
    size: AbstractValueSize,
}

impl<'a> AbstractValueSizeVisitor<'a> {
    fn new(params: &'a AbstractValueSizeGasParameters) -> Self {
        Self {
            params,
            size: 0.into(),
        }
    }

    fn finish(self) -> AbstractValueSize {
        self.size
    }
}

impl<'a> ValueVisitor for AbstractValueSizeVisitor<'a> {
    #[inline]
    fn visit_u8(&mut self, _depth: usize, _val: u8) {
        self.size += self.params.u8;
    }

    #[inline]
    fn visit_u64(&mut self, _depth: usize, _val: u64) {
        self.size += self.params.u64;
    }

    #[inline]
    fn visit_u128(&mut self, _depth: usize, _val: u128) {
        self.size += self.params.u128;
    }

    #[inline]
    fn visit_bool(&mut self, _depth: usize, _val: bool) {
        self.size += self.params.bool;
    }

    #[inline]
    fn visit_address(&mut self, _depth: usize, _val: AccountAddress) {
        self.size += self.params.address;
    }

    #[inline]
    fn visit_struct(&mut self, _depth: usize, _len: usize) -> bool {
        self.size += self.params.struct_;
        true
    }

    #[inline]
    fn visit_vec(&mut self, _depth: usize, _len: usize) -> bool {
        self.size += self.params.vector;
        true
    }

    #[inline]
    fn visit_vec_u8(&mut self, _depth: usize, vals: &[u8]) {
        self.size += self.params.per_u8_packed * NumArgs::new(vals.len() as u64);
    }

    #[inline]
    fn visit_vec_u64(&mut self, _depth: usize, vals: &[u64]) {
        self.size += self.params.per_u64_packed * NumArgs::new(vals.len() as u64);
    }

    #[inline]
    fn visit_vec_u128(&mut self, _depth: usize, vals: &[u128]) {
        self.size += self.params.per_u128_packed * NumArgs::new(vals.len() as u64);
    }

    #[inline]
    fn visit_vec_bool(&mut self, _depth: usize, vals: &[bool]) {
        self.size += self.params.per_bool_packed * NumArgs::new(vals.len() as u64);
    }

    #[inline]
    fn visit_vec_address(&mut self, _depth: usize, vals: &[AccountAddress]) {
        self.size += self.params.per_address_packed * NumArgs::new(vals.len() as u64);
    }

    #[inline]
    fn visit_ref(&mut self, _depth: usize, _is_global: bool) -> bool {
        self.size += self.params.reference;
        false
    }
}

impl AbstractValueSizeGasParameters {
    /// Calculates the abstract size of the given value.
    pub fn abstract_value_size(&self, val: impl ValueView) -> AbstractValueSize {
        let mut visitor = AbstractValueSizeVisitor::new(self);
        val.visit(&mut visitor);
        visitor.finish()
    }

    /// Calculates the abstract size of the given value.
    /// If the value is a reference, then the size of the value behind it will be returned.
    pub fn abstract_value_size_dereferenced(&self, val: impl ValueView) -> AbstractValueSize {
        let mut visitor = DerefVisitor::new(AbstractValueSizeVisitor::new(self));
        val.visit(&mut visitor);
        visitor.into_inner().finish()
    }
}

/// Miscellaneous gas parameters.
#[derive(Debug, Clone)]
pub struct MiscGasParameters {
    pub abs_val: AbstractValueSizeGasParameters,
}

impl FromOnChainGasSchedule for MiscGasParameters {
    fn from_on_chain_gas_schedule(gas_schedule: &BTreeMap<String, u64>) -> Option<Self> {
        Some(Self {
            abs_val: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule)?,
        })
    }
}

impl ToOnChainGasSchedule for MiscGasParameters {
    fn to_on_chain_gas_schedule(&self) -> Vec<(String, u64)> {
        self.abs_val.to_on_chain_gas_schedule()
    }
}

impl MiscGasParameters {
    pub fn zeros() -> Self {
        Self {
            abs_val: AbstractValueSizeGasParameters::zeros(),
        }
    }
}

impl InitialGasSchedule for MiscGasParameters {
    fn initial() -> Self {
        Self {
            abs_val: InitialGasSchedule::initial(),
        }
    }
}
