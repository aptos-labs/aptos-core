// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove's feature gaps: errors MonoMove reports where V1 runs the input.

use crate::v1_error::{describe, V1Equivalent};
use mono_move_core::{storage::resource_provider::ResourceProviderError, VMInternalError};
use mono_move_loader::LoaderError;
use mono_move_runtime::RuntimeError;
use move_core_types::account_address::AccountAddress;

/// A feature V1 has and MonoMove does not implement yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    pub kind: GapKind,
    /// What the error reported.
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GapKind {
    /// A native V1 implements and MonoMove's native registry lacks.
    MissingNative {
        address: AccountAddress,
        module: String,
        function: String,
    },
    /// A construct the lowering does not handle yet.
    LoweringSkipped { reason: &'static str },
    /// A resource type whose layout MonoMove cannot derive outside lowered code.
    ResourceLayoutNotDerivable,
    /// A runtime operation MonoMove rejects.
    RuntimeUnsupported { what: &'static str },
    /// A gap [`describe`] reports that none of the kinds above covers.
    Other,
}

/// The gap `err` reports, if it is one. Gaps are exactly the errors [`describe`] maps to
/// [`V1Equivalent::NoV1Failure`], so the two cannot disagree.
pub fn gap(err: &VMInternalError) -> Option<Gap> {
    if !matches!(describe(err), V1Equivalent::NoV1Failure) {
        return None;
    }
    let (kind, message) = if let Some(err) = err.downcast_ref::<LoaderError>() {
        let kind = match err {
            LoaderError::NativeFunctionNotLoadable {
                address,
                module,
                name,
            } => GapKind::MissingNative {
                address: *address,
                module: module.clone(),
                function: name.clone(),
            },
            LoaderError::LoweringSkipped { reason } => {
                return Some(Gap {
                    kind: GapKind::LoweringSkipped { reason },
                    message: reason.to_string(),
                });
            },
            LoaderError::ResourceLayoutNotDerivable => GapKind::ResourceLayoutNotDerivable,
            LoaderError::ModuleNotFound { .. }
            | LoaderError::FunctionNotFound { .. }
            | LoaderError::ScriptDeserializationFailed { .. }
            | LoaderError::ScriptVerificationFailed { .. }
            | LoaderError::GlobalContext(_)
            | LoaderError::InvariantViolation(_) => GapKind::Other,
        };
        (kind, err.to_string())
    } else if let Some(
        RuntimeError::Unsupported(what)
        | RuntimeError::ResourceProvider(ResourceProviderError::Unsupported(what)),
    ) = err.downcast_ref::<RuntimeError>()
    {
        (GapKind::RuntimeUnsupported { what }, what.to_string())
    } else if let Some(RuntimeError::ArgumentStorageRead(inner)) =
        err.downcast_ref::<RuntimeError>()
    {
        // An argument's storage read hit the gap; it is classified by what the read reported.
        return gap(inner);
    } else {
        (GapKind::Other, err.to_string())
    };
    Some(Gap { kind, message })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_loader::LoaderInvariantViolation;
    use move_binary_format::errors::{Location, PartialVMError};
    use move_core_types::vm_status::StatusCode;

    #[test]
    fn every_gap_has_a_kind_and_nothing_else_is_a_gap() {
        let address = AccountAddress::ONE;
        let loader_cases = [
            LoaderError::ModuleNotFound {
                address,
                name: "m".to_string(),
            },
            LoaderError::FunctionNotFound {
                address,
                module: "m".to_string(),
                name: "f".to_string(),
            },
            LoaderError::NativeFunctionNotLoadable {
                address,
                module: "m".to_string(),
                name: "f".to_string(),
            },
            LoaderError::LoweringSkipped { reason: "nominal" },
            LoaderError::ResourceLayoutNotDerivable,
            LoaderError::ScriptDeserializationFailed {
                message: "truncated".to_string(),
            },
            LoaderError::ScriptVerificationFailed {
                error: PartialVMError::new(StatusCode::MISSING_DEPENDENCY).finish(Location::Script),
            },
            LoaderError::GlobalContext(std::fmt::Error.into()),
            LoaderError::InvariantViolation(LoaderInvariantViolation::EntryAlreadyExists),
        ];
        let errors = loader_cases.into_iter().map(VMInternalError::new).chain([
            VMInternalError::new(RuntimeError::Unsupported("closures")),
            VMInternalError::new(RuntimeError::ResourceProvider(
                ResourceProviderError::Unsupported("function values"),
            )),
            VMInternalError::new(RuntimeError::ResourceProvider(
                ResourceProviderError::InvariantViolation("bad".to_string()),
            )),
            VMInternalError::new(RuntimeError::StackOverflow),
            VMInternalError::new(RuntimeError::ArgumentStorageRead(VMInternalError::new(
                RuntimeError::ResourceProvider(ResourceProviderError::Unsupported("enums")),
            ))),
        ]);
        let mut kinds = vec![];
        for err in errors {
            let is_gap = matches!(describe(&err), V1Equivalent::NoV1Failure);
            match gap(&err) {
                Some(gap) => {
                    assert!(is_gap, "{err}");
                    assert_ne!(gap.kind, GapKind::Other, "{err} needs a gap kind");
                    kinds.push(gap.kind);
                },
                None => assert!(!is_gap, "{err}"),
            }
        }
        assert_eq!(kinds, vec![
            GapKind::MissingNative {
                address,
                module: "m".to_string(),
                function: "f".to_string(),
            },
            GapKind::LoweringSkipped { reason: "nominal" },
            GapKind::ResourceLayoutNotDerivable,
            GapKind::RuntimeUnsupported { what: "closures" },
            GapKind::RuntimeUnsupported {
                what: "function values",
            },
            GapKind::RuntimeUnsupported { what: "enums" },
        ]);
    }
}
