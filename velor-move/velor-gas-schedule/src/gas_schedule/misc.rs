// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the miscellaneous gas parameters, currently only including the
//! ones related to definition of abstract value size.

use crate::{
    gas_schedule::VMGasParameters,
    traits::{FromOnChainGasSchedule, InitialGasSchedule, ToOnChainGasSchedule},
    ver::gas_feature_versions::RELEASE_V1_33,
};
use velor_gas_algebra::{AbstractValueSize, AbstractValueSizePerArg};
use move_core_types::{
    account_address::AccountAddress, gas_algebra::NumArgs, u256::U256, vm_status::StatusCode,
};
use move_vm_types::{
    delayed_values::delayed_field_id::DelayedFieldID,
    natives::function::{PartialVMError, PartialVMResult},
    values::DEFAULT_MAX_VM_VALUE_NESTED_DEPTH,
    views::{ValueView, ValueVisitor},
};
use std::collections::BTreeMap;

crate::gas_schedule::macros::define_gas_parameters!(
    AbstractValueSizeGasParameters,
    "misc.abs_val",
    VMGasParameters => .misc.abs_val,
    [
        // abstract value size
        [u8: AbstractValueSize, "u8", 40],
        [u16: AbstractValueSize, { 5.. => "u16" }, 40],
        [u32: AbstractValueSize, { 5.. => "u32" }, 40],
        [u64: AbstractValueSize, "u64", 40],
        [u128: AbstractValueSize, "u128", 40],
        [u256: AbstractValueSize, { 5.. => "u256" }, 40],
        [bool: AbstractValueSize, "bool", 40],
        [address: AbstractValueSize, "address", 40],
        [struct_: AbstractValueSize, "struct", 40],
        [closure: AbstractValueSize, { RELEASE_V1_33.. => "closure" }, 40],
        [vector: AbstractValueSize, "vector", 40],
        [reference: AbstractValueSize, "reference", 40],
        [per_u8_packed: AbstractValueSizePerArg, "per_u8_packed", 1],
        [per_u16_packed: AbstractValueSizePerArg, { 5.. => "per_u16_packed" }, 2],
        [per_u32_packed: AbstractValueSizePerArg, { 5.. => "per_u32_packed" }, 4],
        [per_u64_packed: AbstractValueSizePerArg, "per_u64_packed", 8],
        [
            per_u128_packed: AbstractValueSizePerArg,
            "per_u128_packed",
            16
        ],
        [per_u256_packed: AbstractValueSizePerArg, { 5.. => "per_u256_packed" }, 32],
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
            fn $fn(&mut self, depth: u64, val: $ty) -> PartialVMResult<()> {
                self.inner.$fn(depth - self.offset as u64, val)?;
                Ok(())
            }
        )*
    };
}

impl<V> ValueVisitor for DerefVisitor<V>
where
    V: ValueVisitor,
{
    deref_visitor_delegate_simple!(
        [visit_delayed, DelayedFieldID],
        [visit_u8, u8],
        [visit_u16, u16],
        [visit_u32, u32],
        [visit_u64, u64],
        [visit_u128, u128],
        [visit_u256, U256],
        [visit_bool, bool],
        [visit_address, AccountAddress],
        [visit_vec_u8, &[u8]],
        [visit_vec_u64, &[u64]],
        [visit_vec_u128, &[u128]],
        [visit_vec_bool, &[bool]],
        [visit_vec_address, &[AccountAddress]],
    );

    #[inline]
    fn visit_struct(&mut self, depth: u64, len: usize) -> PartialVMResult<bool> {
        self.inner.visit_struct(depth - self.offset as u64, len)
    }

    #[inline]
    fn visit_vec(&mut self, depth: u64, len: usize) -> PartialVMResult<bool> {
        self.inner.visit_vec(depth - self.offset as u64, len)
    }

    #[inline]
    fn visit_ref(&mut self, depth: u64, _is_global: bool) -> PartialVMResult<bool> {
        assert_eq!(depth, 0, "There shouldn't be inner refs");
        self.offset = 1;
        Ok(true)
    }

    #[inline]
    fn visit_closure(&mut self, depth: u64, len: usize) -> PartialVMResult<bool> {
        self.inner.visit_closure(depth, len)
    }
}

/// Checks that the provided depth is not too deep. Used to bound recursion, preventing stack from
/// overflowing.
macro_rules! check_depth_impl {
    () => {
        fn check_depth(&self, depth: u64) -> PartialVMResult<()> {
            if self
                .max_value_nest_depth
                .map_or(false, |max_value_nest_depth| depth > max_value_nest_depth)
            {
                return Err(PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED));
            }
            Ok(())
        }
    };
}

struct AbstractValueSizeVisitor<'a> {
    feature_version: u64,
    params: &'a AbstractValueSizeGasParameters,
    size: AbstractValueSize,
    max_value_nest_depth: Option<u64>,
}

impl<'a> AbstractValueSizeVisitor<'a> {
    check_depth_impl!();

    fn new(params: &'a AbstractValueSizeGasParameters, feature_version: u64) -> Self {
        Self {
            feature_version,
            params,
            size: 0.into(),
            max_value_nest_depth: Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH),
        }
    }

    fn finish(self) -> AbstractValueSize {
        self.size
    }
}

impl ValueVisitor for AbstractValueSizeVisitor<'_> {
    #[inline]
    fn visit_delayed(&mut self, depth: u64, _id: DelayedFieldID) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size += self.params.u64;
        Ok(())
    }

    #[inline]
    fn visit_u8(&mut self, depth: u64, _val: u8) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size += self.params.u8;
        Ok(())
    }

    #[inline]
    fn visit_u16(&mut self, depth: u64, _val: u16) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size += self.params.u16;
        Ok(())
    }

    #[inline]
    fn visit_u32(&mut self, depth: u64, _val: u32) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size += self.params.u32;
        Ok(())
    }

    #[inline]
    fn visit_u64(&mut self, depth: u64, _val: u64) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size += self.params.u64;
        Ok(())
    }

    #[inline]
    fn visit_u128(&mut self, depth: u64, _val: u128) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size += self.params.u128;
        Ok(())
    }

    #[inline]
    fn visit_u256(&mut self, depth: u64, _val: U256) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size += self.params.u256;
        Ok(())
    }

    #[inline]
    fn visit_bool(&mut self, depth: u64, _val: bool) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size += self.params.bool;
        Ok(())
    }

    #[inline]
    fn visit_address(&mut self, depth: u64, _val: AccountAddress) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size += self.params.address;
        Ok(())
    }

    #[inline]
    fn visit_struct(&mut self, depth: u64, _len: usize) -> PartialVMResult<bool> {
        self.check_depth(depth)?;
        self.size += self.params.struct_;
        Ok(true)
    }

    #[inline]
    fn visit_closure(&mut self, depth: u64, _len: usize) -> PartialVMResult<bool> {
        self.check_depth(depth)?;
        self.size += self.params.closure;
        Ok(true)
    }

    #[inline]
    fn visit_vec(&mut self, depth: u64, _len: usize) -> PartialVMResult<bool> {
        self.check_depth(depth)?;
        self.size += self.params.vector;
        Ok(true)
    }

    #[inline]
    fn visit_vec_u8(&mut self, depth: u64, vals: &[u8]) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        let mut size = self.params.per_u8_packed * NumArgs::new(vals.len() as u64);
        if self.feature_version >= 3 {
            size += self.params.vector;
        }
        self.size += size;
        Ok(())
    }

    #[inline]
    fn visit_vec_u16(&mut self, depth: u64, vals: &[u16]) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size +=
            self.params.vector + self.params.per_u16_packed * NumArgs::new(vals.len() as u64);
        Ok(())
    }

    #[inline]
    fn visit_vec_u32(&mut self, depth: u64, vals: &[u32]) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size +=
            self.params.vector + self.params.per_u32_packed * NumArgs::new(vals.len() as u64);
        Ok(())
    }

    #[inline]
    fn visit_vec_u64(&mut self, depth: u64, vals: &[u64]) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        let mut size = self.params.per_u64_packed * NumArgs::new(vals.len() as u64);
        if self.feature_version >= 3 {
            size += self.params.vector;
        }
        self.size += size;
        Ok(())
    }

    #[inline]
    fn visit_vec_u128(&mut self, depth: u64, vals: &[u128]) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        let mut size = self.params.per_u128_packed * NumArgs::new(vals.len() as u64);
        if self.feature_version >= 3 {
            size += self.params.vector;
        }
        self.size += size;
        Ok(())
    }

    #[inline]
    fn visit_vec_u256(&mut self, depth: u64, vals: &[U256]) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        self.size +=
            self.params.vector + self.params.per_u256_packed * NumArgs::new(vals.len() as u64);
        Ok(())
    }

    #[inline]
    fn visit_vec_bool(&mut self, depth: u64, vals: &[bool]) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        let mut size = self.params.per_bool_packed * NumArgs::new(vals.len() as u64);
        if self.feature_version >= 3 {
            size += self.params.vector;
        }
        self.size += size;
        Ok(())
    }

    #[inline]
    fn visit_vec_address(&mut self, depth: u64, vals: &[AccountAddress]) -> PartialVMResult<()> {
        self.check_depth(depth)?;
        let mut size = self.params.per_address_packed * NumArgs::new(vals.len() as u64);
        if self.feature_version >= 3 {
            size += self.params.vector;
        }
        self.size += size;
        Ok(())
    }

    #[inline]
    fn visit_ref(&mut self, depth: u64, _is_global: bool) -> PartialVMResult<bool> {
        self.check_depth(depth)?;
        self.size += self.params.reference;
        Ok(false)
    }
}

impl AbstractValueSizeGasParameters {
    /// Calculates the abstract size of the given value.
    pub fn abstract_value_size(
        &self,
        val: impl ValueView,
        feature_version: u64,
    ) -> PartialVMResult<AbstractValueSize> {
        let mut visitor = AbstractValueSizeVisitor::new(self, feature_version);
        val.visit(&mut visitor)?;
        Ok(visitor.finish())
    }

    /// Calculates the abstract size of the given value.
    /// If the value is a reference, then the size of the value behind it will be returned.
    pub fn abstract_value_size_dereferenced(
        &self,
        val: impl ValueView,
        feature_version: u64,
    ) -> PartialVMResult<AbstractValueSize> {
        let mut visitor = DerefVisitor::new(AbstractValueSizeVisitor::new(self, feature_version));
        val.visit(&mut visitor)?;
        Ok(visitor.into_inner().finish())
    }
}

impl AbstractValueSizeGasParameters {
    pub fn abstract_stack_size(
        &self,
        val: impl ValueView,
        feature_version: u64,
    ) -> PartialVMResult<AbstractValueSize> {
        struct Visitor<'a> {
            feature_version: u64,
            params: &'a AbstractValueSizeGasParameters,
            res: Option<AbstractValueSize>,
            max_value_nest_depth: Option<u64>,
        }

        impl Visitor<'_> {
            check_depth_impl!();
        }

        impl ValueVisitor for Visitor<'_> {
            #[inline]
            fn visit_delayed(&mut self, depth: u64, _val: DelayedFieldID) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.u64);
                Ok(())
            }

            #[inline]
            fn visit_u8(&mut self, depth: u64, _val: u8) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.u8);
                Ok(())
            }

            #[inline]
            fn visit_u16(&mut self, depth: u64, _val: u16) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.u16);
                Ok(())
            }

            #[inline]
            fn visit_u32(&mut self, depth: u64, _val: u32) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.u32);
                Ok(())
            }

            #[inline]
            fn visit_u64(&mut self, depth: u64, _val: u64) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.u64);
                Ok(())
            }

            #[inline]
            fn visit_u128(&mut self, depth: u64, _val: u128) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.u128);
                Ok(())
            }

            #[inline]
            fn visit_u256(&mut self, depth: u64, _val: U256) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.u256);
                Ok(())
            }

            #[inline]
            fn visit_bool(&mut self, depth: u64, _val: bool) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.bool);
                Ok(())
            }

            #[inline]
            fn visit_address(&mut self, depth: u64, _val: AccountAddress) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.address);
                Ok(())
            }

            #[inline]
            fn visit_struct(&mut self, depth: u64, _len: usize) -> PartialVMResult<bool> {
                self.check_depth(depth)?;
                self.res = Some(self.params.struct_);
                Ok(false)
            }

            #[inline]
            fn visit_closure(&mut self, depth: u64, _len: usize) -> PartialVMResult<bool> {
                self.check_depth(depth)?;
                self.res = Some(self.params.closure);
                Ok(false)
            }

            #[inline]
            fn visit_vec(&mut self, depth: u64, _len: usize) -> PartialVMResult<bool> {
                self.check_depth(depth)?;
                self.res = Some(self.params.vector);
                Ok(false)
            }

            #[inline]
            fn visit_ref(&mut self, depth: u64, _is_global: bool) -> PartialVMResult<bool> {
                self.check_depth(depth)?;
                self.res = Some(self.params.reference);
                Ok(false)
            }

            // TODO(Gas): The following function impls are necessary due to a bug upstream.
            //            Remove them once the bug is fixed.
            #[inline]
            fn visit_vec_u8(&mut self, depth: u64, vals: &[u8]) -> PartialVMResult<()> {
                if self.feature_version < 3 {
                    self.res = Some(0.into());
                } else {
                    self.visit_vec(depth, vals.len())?;
                }
                Ok(())
            }

            #[inline]
            fn visit_vec_u16(&mut self, depth: u64, vals: &[u16]) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }

            #[inline]
            fn visit_vec_u32(&mut self, depth: u64, vals: &[u32]) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }

            #[inline]
            fn visit_vec_u64(&mut self, depth: u64, vals: &[u64]) -> PartialVMResult<()> {
                if self.feature_version < 3 {
                    self.res = Some(0.into());
                } else {
                    self.visit_vec(depth, vals.len())?;
                }
                Ok(())
            }

            #[inline]
            fn visit_vec_u128(&mut self, depth: u64, vals: &[u128]) -> PartialVMResult<()> {
                if self.feature_version < 3 {
                    self.res = Some(0.into());
                } else {
                    self.visit_vec(depth, vals.len())?;
                }
                Ok(())
            }

            #[inline]
            fn visit_vec_u256(&mut self, depth: u64, vals: &[U256]) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }

            #[inline]
            fn visit_vec_bool(&mut self, depth: u64, vals: &[bool]) -> PartialVMResult<()> {
                if self.feature_version < 3 {
                    self.res = Some(0.into());
                } else {
                    self.visit_vec(depth, vals.len())?;
                }
                Ok(())
            }

            #[inline]
            fn visit_vec_address(
                &mut self,
                depth: u64,
                vals: &[AccountAddress],
            ) -> PartialVMResult<()> {
                if self.feature_version < 3 {
                    self.res = Some(0.into());
                } else {
                    self.visit_vec(depth, vals.len())?;
                }
                Ok(())
            }
        }

        let mut visitor = Visitor {
            feature_version,
            params: self,
            res: None,
            max_value_nest_depth: Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH),
        };
        val.visit(&mut visitor)?;
        visitor.res.ok_or_else(|| {
            PartialVMError::new_invariant_violation("Visitor should have set the `res` value")
        })
    }

    pub fn abstract_packed_size(&self, val: impl ValueView) -> PartialVMResult<AbstractValueSize> {
        struct Visitor<'a> {
            params: &'a AbstractValueSizeGasParameters,
            res: Option<AbstractValueSize>,
            max_value_nest_depth: Option<u64>,
        }

        impl Visitor<'_> {
            check_depth_impl!();
        }

        impl ValueVisitor for Visitor<'_> {
            #[inline]
            fn visit_delayed(&mut self, depth: u64, _val: DelayedFieldID) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.per_u64_packed * NumArgs::from(1));
                Ok(())
            }

            #[inline]
            fn visit_u8(&mut self, depth: u64, _val: u8) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.per_u8_packed * NumArgs::from(1));
                Ok(())
            }

            #[inline]
            fn visit_u16(&mut self, depth: u64, _val: u16) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.per_u16_packed * NumArgs::from(1));
                Ok(())
            }

            #[inline]
            fn visit_u32(&mut self, depth: u64, _val: u32) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.per_u32_packed * NumArgs::from(1));
                Ok(())
            }

            #[inline]
            fn visit_u64(&mut self, depth: u64, _val: u64) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.per_u64_packed * NumArgs::from(1));
                Ok(())
            }

            #[inline]
            fn visit_u128(&mut self, depth: u64, _val: u128) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.per_u128_packed * NumArgs::from(1));
                Ok(())
            }

            #[inline]
            fn visit_u256(&mut self, depth: u64, _val: U256) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.per_u256_packed * NumArgs::from(1));
                Ok(())
            }

            #[inline]
            fn visit_bool(&mut self, depth: u64, _val: bool) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.per_bool_packed * NumArgs::from(1));
                Ok(())
            }

            #[inline]
            fn visit_address(&mut self, depth: u64, _val: AccountAddress) -> PartialVMResult<()> {
                self.check_depth(depth)?;
                self.res = Some(self.params.per_address_packed * NumArgs::from(1));
                Ok(())
            }

            #[inline]
            fn visit_struct(&mut self, depth: u64, _len: usize) -> PartialVMResult<bool> {
                self.check_depth(depth)?;
                self.res = Some(self.params.struct_);
                Ok(false)
            }

            #[inline]
            fn visit_closure(&mut self, depth: u64, _len: usize) -> PartialVMResult<bool> {
                self.check_depth(depth)?;
                self.res = Some(self.params.closure);
                Ok(false)
            }

            #[inline]
            fn visit_vec(&mut self, depth: u64, _len: usize) -> PartialVMResult<bool> {
                self.check_depth(depth)?;
                self.res = Some(self.params.vector);
                Ok(false)
            }

            #[inline]
            fn visit_ref(&mut self, depth: u64, _is_global: bool) -> PartialVMResult<bool> {
                // TODO(Gas): This should be unreachable...
                //            See if we can handle this in a more graceful way.
                self.check_depth(depth)?;
                self.res = Some(self.params.reference);
                Ok(false)
            }

            // TODO(Gas): The following function impls are necessary due to a bug upstream.
            //            Remove them once the bug is fixed.
            #[inline]
            fn visit_vec_u8(&mut self, depth: u64, vals: &[u8]) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }

            #[inline]
            fn visit_vec_u16(&mut self, depth: u64, vals: &[u16]) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }

            #[inline]
            fn visit_vec_u32(&mut self, depth: u64, vals: &[u32]) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }

            #[inline]
            fn visit_vec_u64(&mut self, depth: u64, vals: &[u64]) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }

            #[inline]
            fn visit_vec_u128(&mut self, depth: u64, vals: &[u128]) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }

            fn visit_vec_u256(&mut self, depth: u64, vals: &[U256]) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }

            #[inline]
            fn visit_vec_bool(&mut self, depth: u64, vals: &[bool]) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }

            #[inline]
            fn visit_vec_address(
                &mut self,
                depth: u64,
                vals: &[AccountAddress],
            ) -> PartialVMResult<()> {
                self.visit_vec(depth, vals.len())?;
                Ok(())
            }
        }

        let mut visitor = Visitor {
            params: self,
            res: None,
            max_value_nest_depth: Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH),
        };
        val.visit(&mut visitor)?;
        visitor.res.ok_or_else(|| {
            PartialVMError::new_invariant_violation("Visitor should have set the `res` value")
        })
    }

    pub fn abstract_value_size_stack_and_heap(
        &self,
        val: impl ValueView,
        feature_version: u64,
    ) -> PartialVMResult<(AbstractValueSize, AbstractValueSize)> {
        let stack_size = self.abstract_stack_size(&val, feature_version)?;
        let abs_size = self.abstract_value_size(val, feature_version)?;
        let heap_size = abs_size.checked_sub(stack_size).unwrap_or_else(|| 0.into());

        Ok((stack_size, heap_size))
    }

    pub fn abstract_heap_size(
        &self,
        val: impl ValueView,
        feature_version: u64,
    ) -> PartialVMResult<AbstractValueSize> {
        let stack_size = self.abstract_stack_size(&val, feature_version)?;
        let abs_size = self.abstract_value_size(val, feature_version)?;

        Ok(abs_size.checked_sub(stack_size).unwrap_or_else(|| 0.into()))
    }
}

/// Miscellaneous gas parameters.
#[derive(Debug, Clone)]
pub struct MiscGasParameters {
    pub abs_val: AbstractValueSizeGasParameters,
}

impl FromOnChainGasSchedule for MiscGasParameters {
    fn from_on_chain_gas_schedule(
        gas_schedule: &BTreeMap<String, u64>,
        feature_version: u64,
    ) -> Result<Self, String> {
        Ok(Self {
            abs_val: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
        })
    }
}

impl ToOnChainGasSchedule for MiscGasParameters {
    fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)> {
        self.abs_val.to_on_chain_gas_schedule(feature_version)
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
