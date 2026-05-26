// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{delayed_values::delayed_field_id::DelayedFieldID, values::LEGACY_CLOSURE_SIZE};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    account_address::AccountAddress, gas_algebra::AbstractMemorySize, language_storage::TypeTag,
};
use std::mem::size_of_val;

/// Trait that provides an abstract view into a Move type.
///
/// This is used to expose certain info to clients (e.g. the gas meter),
/// usually in a lazily evaluated fashion.
pub trait TypeView {
    /// Returns the `TypeTag` (fully qualified name) of the type.
    fn to_type_tag(&self) -> TypeTag;
}

/// Trait that provides an abstract view into a Move Value.
///
/// This is used to expose certain info to clients (e.g. the gas meter),
/// usually in a lazily evaluated fashion.
pub trait ValueView {
    fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()>;

    /// Returns the abstract memory size of the value.
    ///
    /// The concept of abstract memory size is not well-defined and is only kept for backward compatibility.
    /// New applications should avoid using this.
    ///
    /// TODO(Gas): Encourage clients to replicate this in their own repo and get this removed once
    ///            they are done.
    fn legacy_abstract_memory_size(&self) -> AbstractMemorySize {
        use crate::values::{LEGACY_CONST_SIZE, LEGACY_REFERENCE_SIZE, LEGACY_STRUCT_SIZE};

        struct Acc(AbstractMemorySize);

        impl ValueVisitor for Acc {
            fn visit_delayed(&mut self, _depth: u64, _id: DelayedFieldID) -> PartialVMResult<()> {
                // TODO[agg_v2](cleanup): `legacy_abstract_memory_size` is not used
                //   anyway, so this function will be removed soon (hopefully).
                //   Contributions are appreciated!
                Ok(())
            }

            fn visit_u8(&mut self, _depth: u64, _val: u8) -> PartialVMResult<()> {
                self.0 += LEGACY_CONST_SIZE;
                Ok(())
            }

            fn visit_u16(&mut self, _depth: u64, _val: u16) -> PartialVMResult<()> {
                self.0 += LEGACY_CONST_SIZE;
                Ok(())
            }

            fn visit_u32(&mut self, _depth: u64, _val: u32) -> PartialVMResult<()> {
                self.0 += LEGACY_CONST_SIZE;
                Ok(())
            }

            fn visit_u64(&mut self, _depth: u64, _val: u64) -> PartialVMResult<()> {
                self.0 += LEGACY_CONST_SIZE;
                Ok(())
            }

            fn visit_u128(&mut self, _depth: u64, _val: u128) -> PartialVMResult<()> {
                self.0 += LEGACY_CONST_SIZE;
                Ok(())
            }

            fn visit_u256(
                &mut self,
                _depth: u64,
                _val: move_core_types::u256::U256,
            ) -> PartialVMResult<()> {
                self.0 += LEGACY_CONST_SIZE;
                Ok(())
            }

            fn visit_bool(&mut self, _depth: u64, _val: bool) -> PartialVMResult<()> {
                self.0 += LEGACY_CONST_SIZE;
                Ok(())
            }

            fn visit_address(&mut self, _depth: u64, _val: AccountAddress) -> PartialVMResult<()> {
                self.0 += AbstractMemorySize::new(AccountAddress::LENGTH as u64);
                Ok(())
            }

            fn visit_struct(&mut self, _depth: u64, _len: usize) -> PartialVMResult<bool> {
                self.0 += LEGACY_STRUCT_SIZE;
                Ok(true)
            }

            fn visit_closure(&mut self, _depth: u64, _len: usize) -> PartialVMResult<bool> {
                self.0 += LEGACY_CLOSURE_SIZE;
                Ok(true)
            }

            fn visit_vec(&mut self, _depth: u64, _len: usize) -> PartialVMResult<bool> {
                self.0 += LEGACY_STRUCT_SIZE;
                Ok(true)
            }

            fn visit_vec_u8(&mut self, _depth: u64, vals: &[u8]) -> PartialVMResult<()> {
                self.0 += (size_of_val(vals) as u64).into();
                Ok(())
            }

            fn visit_vec_u16(&mut self, _depth: u64, vals: &[u16]) -> PartialVMResult<()> {
                self.0 += (size_of_val(vals) as u64).into();
                Ok(())
            }

            fn visit_vec_u32(&mut self, _depth: u64, vals: &[u32]) -> PartialVMResult<()> {
                self.0 += (size_of_val(vals) as u64).into();
                Ok(())
            }

            fn visit_vec_u64(&mut self, _depth: u64, vals: &[u64]) -> PartialVMResult<()> {
                self.0 += (size_of_val(vals) as u64).into();
                Ok(())
            }

            fn visit_vec_u128(&mut self, _depth: u64, vals: &[u128]) -> PartialVMResult<()> {
                self.0 += (size_of_val(vals) as u64).into();
                Ok(())
            }

            fn visit_vec_u256(
                &mut self,
                _depth: u64,
                vals: &[move_core_types::u256::U256],
            ) -> PartialVMResult<()> {
                self.0 += (size_of_val(vals) as u64).into();
                Ok(())
            }

            fn visit_vec_bool(&mut self, _depth: u64, vals: &[bool]) -> PartialVMResult<()> {
                self.0 += (size_of_val(vals) as u64).into();
                Ok(())
            }

            fn visit_vec_address(
                &mut self,
                _depth: u64,
                vals: &[AccountAddress],
            ) -> PartialVMResult<()> {
                self.0 += (size_of_val(vals) as u64).into();
                Ok(())
            }

            fn visit_ref(&mut self, _depth: u64, _is_global: bool) -> PartialVMResult<bool> {
                self.0 += LEGACY_REFERENCE_SIZE;
                Ok(false)
            }
        }

        let mut acc = Acc(0.into());
        self.visit(&mut acc)
            .expect("Legacy function: should not fail");

        acc.0
    }
}

/// Trait that defines a visitor that could be used to traverse a value recursively.
pub trait ValueVisitor {
    fn visit_delayed(&mut self, depth: u64, id: DelayedFieldID) -> PartialVMResult<()>;
    fn visit_u8(&mut self, depth: u64, val: u8) -> PartialVMResult<()>;
    fn visit_u16(&mut self, depth: u64, val: u16) -> PartialVMResult<()>;
    fn visit_u32(&mut self, depth: u64, val: u32) -> PartialVMResult<()>;
    fn visit_u64(&mut self, depth: u64, val: u64) -> PartialVMResult<()>;
    fn visit_u128(&mut self, depth: u64, val: u128) -> PartialVMResult<()>;
    fn visit_u256(&mut self, depth: u64, val: move_core_types::u256::U256) -> PartialVMResult<()>;
    fn visit_bool(&mut self, depth: u64, val: bool) -> PartialVMResult<()>;
    fn visit_address(&mut self, depth: u64, val: AccountAddress) -> PartialVMResult<()>;
    fn visit_struct(&mut self, depth: u64, len: usize) -> PartialVMResult<bool>;
    fn visit_closure(&mut self, depth: u64, len: usize) -> PartialVMResult<bool>;
    fn visit_vec(&mut self, depth: u64, len: usize) -> PartialVMResult<bool>;
    fn visit_ref(&mut self, depth: u64, is_global: bool) -> PartialVMResult<bool>;

    fn visit_vec_u8(&mut self, depth: u64, vals: &[u8]) -> PartialVMResult<()> {
        self.visit_vec(depth, vals.len())?;
        for val in vals {
            self.visit_u8(depth + 1, *val)?;
        }
        Ok(())
    }

    fn visit_vec_u16(&mut self, depth: u64, vals: &[u16]) -> PartialVMResult<()> {
        self.visit_vec(depth, vals.len())?;
        for val in vals {
            self.visit_u16(depth + 1, *val)?;
        }
        Ok(())
    }

    fn visit_vec_u32(&mut self, depth: u64, vals: &[u32]) -> PartialVMResult<()> {
        self.visit_vec(depth, vals.len())?;
        for val in vals {
            self.visit_u32(depth + 1, *val)?;
        }
        Ok(())
    }

    fn visit_vec_u64(&mut self, depth: u64, vals: &[u64]) -> PartialVMResult<()> {
        self.visit_vec(depth, vals.len())?;
        for val in vals {
            self.visit_u64(depth + 1, *val)?;
        }
        Ok(())
    }

    fn visit_vec_u128(&mut self, depth: u64, vals: &[u128]) -> PartialVMResult<()> {
        self.visit_vec(depth, vals.len())?;
        for val in vals {
            self.visit_u128(depth + 1, *val)?;
        }
        Ok(())
    }

    fn visit_vec_u256(
        &mut self,
        depth: u64,
        vals: &[move_core_types::u256::U256],
    ) -> PartialVMResult<()> {
        self.visit_vec(depth, vals.len())?;
        for val in vals {
            self.visit_u256(depth + 1, *val)?;
        }
        Ok(())
    }

    fn visit_vec_bool(&mut self, depth: u64, vals: &[bool]) -> PartialVMResult<()> {
        self.visit_vec(depth, vals.len())?;
        for val in vals {
            self.visit_bool(depth + 1, *val)?;
        }
        Ok(())
    }

    fn visit_vec_address(&mut self, depth: u64, vals: &[AccountAddress]) -> PartialVMResult<()> {
        self.visit_vec(depth, vals.len())?;
        for val in vals {
            self.visit_address(depth + 1, *val)?;
        }
        Ok(())
    }
}

impl<T> ValueView for &T
where
    T: ValueView,
{
    fn legacy_abstract_memory_size(&self) -> AbstractMemorySize {
        <T as ValueView>::legacy_abstract_memory_size(*self)
    }

    fn visit(&self, visitor: &mut impl ValueVisitor) -> PartialVMResult<()> {
        <T as ValueView>::visit(*self, visitor)
    }
}

impl<T> TypeView for &T
where
    T: TypeView,
{
    fn to_type_tag(&self) -> TypeTag {
        <T as TypeView>::to_type_tag(*self)
    }
}
