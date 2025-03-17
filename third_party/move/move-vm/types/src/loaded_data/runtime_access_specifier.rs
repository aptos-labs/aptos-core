// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Runtime representation of access control specifiers.
//!
//! Specifiers are represented as a list of inclusion and exclusion clauses. Each
//! of those clauses corresponds to an `acquires A`, `reads A`, or `writes A`
//! declaration in the language. Exclusions stem from negation, e.g. `!reads A`.
//!
//! Specifiers support access check via `AccessSpecifier::enables`. Moreover,
//! access specifiers can be joined via `AccessSpecifier::join`. The join of two access
//! specifiers behaves like intersection: for `a1 join a2`, access is allowed if it
//! is both allowed by `a1` and `a2`. Joining happens when a function is entered which
//! has access specifiers: then the current active access specifier is joined with the
//! function's specifier. The join operator is complete (no approximation). A further
//! operator `AccessSpecifier::subsumes` allows to test whether one specifier
//! allows all the access of the other. This used to abort execution if a function
//! is entered which declares accesses not allowed by the context. However, the
//!`subsumes` function is incomplete. This is semantically sound since
//! if subsume is undecided, abortion only happens later at the time of actual access
//! instead of when the function is entered.
//!
//! The `join` operation attempts to simplify the resulting access specifier, making
//! access checks faster and keeping memory use low. This is only implemented for
//! inclusions, which are fully simplified. Exclusions are accumulated.
//! There is potential for optimization by simplifying exclusions but since those are effectively
//! negations, such a simplification is not trivial and may require recursive specifiers, which
//! we like to avoid.

use crate::{
    loaded_data::runtime_types::{StructIdentifier, Type},
    values::{Reference, SignerRef, Value},
};
use itertools::Itertools;
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::{AccessKind, LocalIndex},
};
use move_core_types::{
    account_address::AccountAddress, language_storage::ModuleId, vm_status::StatusCode,
};
use std::{fmt, fmt::Debug};

/// Represents an access specifier.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub enum AccessSpecifier {
    /// Universal access granted
    Any,
    /// A constraint in normalized form `Constraint(inclusions, exclusions)`.
    /// The inclusions are a _disjunction_ and the exclusions a _conjunction_ of
    /// access clauses. An access is valid if it is enabled by any of the
    /// inclusions, and not enabled for each of the exclusions.
    Constraint(Vec<AccessSpecifierClause>, Vec<AccessSpecifierClause>),
}

/// Represents an access specifier clause
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub struct AccessSpecifierClause {
    pub kind: AccessKind,
    pub resource: ResourceSpecifier,
    pub address: AddressSpecifier,
}

/// Represents a resource specifier.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub enum ResourceSpecifier {
    Any,
    DeclaredAtAddress(AccountAddress),
    DeclaredInModule(ModuleId),
    Resource(StructIdentifier),
    ResourceInstantiation(StructIdentifier, Vec<Type>),
}

/// Represents an address specifier.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub enum AddressSpecifier {
    Any,
    Literal(AccountAddress),
    /// The `Eval` specifier represents a value dependent on a parameter of the
    /// current function. Once address specifiers are instantiated in a given
    /// caller context it is replaced by a literal.
    Eval(AddressSpecifierFunction, LocalIndex),
}

/// Represents a well-known function used in an address specifier.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub enum AddressSpecifierFunction {
    /// Identity function -- just returns the value of the parameter.
    Identity,
    /// signer::address_of
    SignerAddress,
    /// object::owner_of
    ObjectAddress,
}

/// A trait representing an environment for evaluating dynamic values in access specifiers.
pub trait AccessSpecifierEnv {
    fn eval_address_specifier_function(
        &self,
        fun: AddressSpecifierFunction,
        local: LocalIndex,
    ) -> PartialVMResult<AccountAddress>;
}

/// A struct to represent an access instance (request).
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub struct AccessInstance {
    pub kind: AccessKind,
    pub resource: StructIdentifier,
    pub instance: Vec<Type>,
    pub address: AccountAddress,
}

impl AccessSpecifier {
    /// Returns true if this access specifier is known to have no accesses. Note that this
    /// may be under-approximated in the presence of exclusions. That is, if
    /// `!s.is_empty()`, it is still possible that all concrete accesses fail.
    pub fn is_empty(&self) -> bool {
        if let AccessSpecifier::Constraint(incls, _) = self {
            incls.is_empty()
        } else {
            false
        }
    }

    /// Specializes the access specifier for the given environment. This evaluates
    /// `AddressSpecifier::Eval` terms.
    pub fn specialize(&mut self, env: &impl AccessSpecifierEnv) -> PartialVMResult<()> {
        match self {
            AccessSpecifier::Any => Ok(()),
            AccessSpecifier::Constraint(incls, excls) => {
                for clause in incls {
                    clause.specialize(env)?;
                }
                for clause in excls {
                    clause.specialize(env)?;
                }
                Ok(())
            },
        }
    }

    /// Returns true if the concrete access instance is enabled.
    pub fn enables(&self, access: &AccessInstance) -> bool {
        use AccessSpecifier::*;
        match self {
            Any => true,
            Constraint(incls, excls) => {
                (incls.is_empty() && !excls.is_empty() || incls.iter().any(|c| c.includes(access)))
                    && excls.iter().all(|c| !c.excludes(access))
            },
        }
    }
}

impl AccessSpecifierClause {
    /// Checks whether this clause allows the access.
    fn includes(&self, access: &AccessInstance) -> bool {
        use AccessKind::*;
        let AccessInstance {
            kind,
            resource,
            instance,
            address,
        } = access;
        let kind_allows = match (self.kind, kind) {
            (Reads, Reads) => true,
            (Reads, Writes) => false,
            // `writes` enables both read and write access
            (Writes, Reads) => true,
            (Writes, Writes) => true,
        };
        kind_allows && self.resource.matches(resource, instance) && self.address.matches(address)
    }

    /// Checks whether this clause disallows the access.
    /// There is a difference in the interpretation of Reads/Writes in negated mode.
    /// With `!reads`, both reading and writing are excluded (since write access also allows
    /// read). With `!writes`, only writing is excluded, while reading is still allowed.
    fn excludes(&self, access: &AccessInstance) -> bool {
        use AccessKind::*;
        let AccessInstance {
            kind,
            resource,
            instance,
            address,
        } = access;
        let kind_excludes = match (self.kind, kind) {
            (Reads, Reads) => true,
            (Reads, Writes) => true,
            (Writes, Reads) => false,
            (Writes, Writes) => true,
        };
        kind_excludes && self.resource.matches(resource, instance) && self.address.matches(address)
    }

    /// Specializes this clause.
    fn specialize(&mut self, env: &impl AccessSpecifierEnv) -> PartialVMResult<()> {
        // Only addresses can be specialized right now.
        self.address.specialize(env)
    }
}

impl ResourceSpecifier {
    /// Checks whether the struct/type pair is enabled by this specifier.
    fn matches(&self, struct_id: &StructIdentifier, type_inst: &[Type]) -> bool {
        use ResourceSpecifier::*;
        match self {
            Any => true,
            DeclaredAtAddress(addr) => struct_id.module.address() == addr,
            DeclaredInModule(module_id) => &struct_id.module == module_id,
            Resource(enabled_struct_id) => enabled_struct_id == struct_id,
            ResourceInstantiation(enabled_struct_id, enabled_type_inst) => {
                enabled_struct_id == struct_id && enabled_type_inst == type_inst
            },
        }
    }
}

impl AddressSpecifier {
    /// Checks whether the given address is enabled by this specifier.
    fn matches(&self, addr: &AccountAddress) -> bool {
        use AddressSpecifier::*;
        match self {
            Any => true,
            Literal(a) => a == addr,
            Eval(_, _) => false,
        }
    }

    /// Specializes this specifier, resolving `Eval` variants.
    fn specialize(&mut self, env: &impl AccessSpecifierEnv) -> PartialVMResult<()> {
        if let AddressSpecifier::Eval(fun, arg) = self {
            *self = AddressSpecifier::Literal(env.eval_address_specifier_function(*fun, *arg)?)
        }
        Ok(())
    }
}

impl AddressSpecifierFunction {
    pub fn parse(module_str: &str, fun_str: &str) -> Option<AddressSpecifierFunction> {
        match (module_str, fun_str) {
            ("0x1::signer", "address_of") => Some(AddressSpecifierFunction::SignerAddress),
            ("0x1::object", "owner") => Some(AddressSpecifierFunction::ObjectAddress),
            _ => None,
        }
    }

    pub fn eval(&self, arg: Value) -> PartialVMResult<AccountAddress> {
        use AddressSpecifierFunction::*;
        match self {
            Identity => arg.value_as::<AccountAddress>(),
            SignerAddress => {
                // See also: implementation of `signer::native_borrow_address`.
                let signer_ref = arg.value_as::<SignerRef>()?;
                signer_ref
                    .borrow_signer()?
                    .value_as::<Reference>()?
                    .read_ref()?
                    .value_as::<AccountAddress>()
            },
            ObjectAddress => Err(PartialVMError::new(
                StatusCode::ACCESS_CONTROL_INVARIANT_VIOLATION,
            )
            .with_message(format!(
                "unimplemented address specifier function `{:?}`",
                self
            ))),
        }
    }
}

impl AccessInstance {
    pub fn new(
        kind: AccessKind,
        resource: StructIdentifier,
        instance: &[Type],
        address: AccountAddress,
    ) -> Option<Self> {
        Some(AccessInstance {
            kind,
            resource,
            instance: instance.to_vec(),
            address,
        })
    }

    pub fn read(
        resource: &StructIdentifier,
        instance: &[Type],
        address: AccountAddress,
    ) -> Option<Self> {
        Self::new(AccessKind::Reads, resource.clone(), instance, address)
    }

    pub fn write(
        resource: &StructIdentifier,
        instance: &[Type],
        address: AccountAddress,
    ) -> Option<Self> {
        Self::new(AccessKind::Writes, resource.clone(), instance, address)
    }
}

impl fmt::Display for AccessInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            kind,
            resource,
            instance,
            address,
        } = self;
        write!(
            f,
            "{} {}{}(@0x{})",
            kind,
            resource,
            if !instance.is_empty() {
                format!("<{}>", instance.iter().map(|t| t.to_string()).join(","))
            } else {
                "".to_owned()
            },
            address.short_str_lossless()
        )
    }
}
