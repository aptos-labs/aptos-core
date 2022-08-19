// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use crate::algebra::{AbstractValueSize, AbstractValueSizePerArg};
use crate::gas_meter::{FromOnChainGasSchedule, InitialGasSchedule, ToOnChainGasSchedule};
use move_core_types::{account_address::AccountAddress, gas_algebra::NumArgs};
use move_vm_types::views::{ValueView, ValueVisitor};

crate::params::define_gas_parameters!(
    AbstractMemorySizeGasParameters,
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

impl AbstractMemorySizeGasParameters {
    pub fn abstract_value_size(&self, val: impl ValueView) -> AbstractValueSize {
        struct Visitor<'a> {
            params: &'a AbstractMemorySizeGasParameters,
            size: AbstractValueSize,
        }

        impl<'a> ValueVisitor for Visitor<'a> {
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

        let mut visitor = Visitor {
            params: self,
            size: 0.into(),
        };
        val.visit(&mut visitor);
        visitor.size
    }
}

#[derive(Debug, Clone)]
pub struct MiscGasParameters {
    pub abs_val: AbstractMemorySizeGasParameters,
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
            abs_val: AbstractMemorySizeGasParameters::zeros(),
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
