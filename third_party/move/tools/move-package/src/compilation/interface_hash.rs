// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Computes a stable hash of a package's public/friend API surface.
//!
//! The hash changes if and only if something observable by dependents changes:
//! public/friend struct or function signatures, friend declarations, and raw
//! module-level metadata bytes (which encode Aptos-specific annotations such as
//! `#[resource_group(scope = ...)]` that affect cross-package validation).
//! Private functions, function bodies, and internal indices are excluded so
//! that implementation-only changes do not force recompilation of dependents.

use crate::source_package::parsed_manifest::PackageDigest;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        FunctionDefinition, SignatureToken, StructDefinition, StructFieldInformation, Visibility,
    },
    CompiledModule,
};
use move_symbol_pool::Symbol;
use sha2::{Digest, Sha256};

/// Compute a stable hash of the public/friend API surface of `modules`.
///
/// The `modules` slice should contain all `CompiledModule`s belonging to a
/// single package. The hash is deterministic: the same set of modules in any
/// order always produces the same value.
///
/// The hash changes iff:
/// - A public/friend function's name, visibility, `is_entry` flag, type
///   parameter constraints, parameter types, or return types change.
/// - A public/friend struct's name, ability set, type parameters, or field
///   layout changes.
/// - A friend declaration is added, removed, or changes target.
/// - Any raw module metadata entry (key or value) changes — this captures
///   Aptos-specific annotations (e.g. `#[resource_group(scope = ...)]`) that
///   affect cross-package extended-check validation.
pub fn compute_interface_hash(modules: &[&CompiledModule]) -> PackageDigest {
    let mut hasher = Sha256::new();
    // Sort by module id for determinism regardless of compilation order.
    let mut sorted: Vec<&&CompiledModule> = modules.iter().collect();
    sorted.sort_by_key(|m| m.self_id().to_string());
    for module in sorted {
        hash_module_interface(&mut hasher, module);
    }
    let bytes = hasher.finalize();
    // Encode as lowercase hex string, then intern as Symbol (PackageDigest = Symbol).
    let hex = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    Symbol::from(hex.as_str())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn hash_str(hasher: &mut Sha256, s: &str) {
    // Length-prefix the string so that ("ab", "c") != ("a", "bc").
    hasher.update((s.len() as u64).to_le_bytes());
    hasher.update(s.as_bytes());
}

fn hash_u8(hasher: &mut Sha256, v: u8) {
    hasher.update([v]);
}

fn hash_bool(hasher: &mut Sha256, v: bool) {
    hasher.update([v as u8]);
}

fn hash_module_interface(hasher: &mut Sha256, module: &CompiledModule) {
    // Module identity
    hash_str(hasher, &module.self_id().to_string());

    // --- Struct definitions (sorted by name) ---
    let mut struct_defs: Vec<&StructDefinition> = module.struct_defs().iter().collect();
    struct_defs.sort_by_key(|sd| {
        module
            .identifier_at(module.struct_handle_at(sd.struct_handle).name)
            .as_str()
    });
    for sd in struct_defs {
        hash_struct_def(hasher, sd, module);
    }

    // --- Public/friend function definitions (sorted by name) ---
    let mut fn_defs: Vec<&FunctionDefinition> = module
        .function_defs()
        .iter()
        .filter(|fd| fd.visibility == Visibility::Public || fd.visibility == Visibility::Friend)
        .collect();
    fn_defs.sort_by_key(|fd| {
        module
            .identifier_at(module.function_handle_at(fd.function).name)
            .as_str()
    });
    for fd in fn_defs {
        hash_function_def(hasher, fd, module);
    }

    // --- Friend declarations (sorted) ---
    let mut friends: Vec<String> = module
        .immediate_friends()
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    friends.sort();
    for f in friends {
        hash_str(hasher, &f);
    }

    // --- Module metadata (sorted by key for determinism) ---
    // Metadata encodes Aptos-specific annotations (e.g. #[resource_group(scope=...)])
    // that affect cross-package extended-check validation. We hash the raw bytes so
    // that any change — including resource group scope changes — invalidates dependents'
    // cached artifacts. Sorting by key ensures a stable ordering regardless of how the
    // compiler writes the entries.
    let mut metadata: Vec<(&[u8], &[u8])> = module
        .metadata
        .iter()
        .map(|m| (m.key.as_slice(), m.value.as_slice()))
        .collect();
    metadata.sort_by_key(|(k, _)| *k);
    for (key, value) in metadata {
        hasher.update((key.len() as u64).to_le_bytes());
        hasher.update(key);
        hasher.update((value.len() as u64).to_le_bytes());
        hasher.update(value);
    }
}

fn hash_struct_def(hasher: &mut Sha256, sd: &StructDefinition, module: &CompiledModule) {
    let handle = module.struct_handle_at(sd.struct_handle);
    let name = module.identifier_at(handle.name);
    hash_str(hasher, name.as_str());

    // Ability set as a byte
    hash_u8(hasher, handle.abilities.into_u8());

    // Type parameters
    for tp in &handle.type_parameters {
        hash_u8(hasher, tp.constraints.into_u8());
        hash_bool(hasher, tp.is_phantom);
    }

    // Fields
    match &sd.field_information {
        StructFieldInformation::Native => hash_str(hasher, "native"),
        StructFieldInformation::Declared(fields) => {
            hash_str(hasher, "declared");
            for field in fields {
                let field_name = module.identifier_at(field.name);
                hash_str(hasher, field_name.as_str());
                hash_sig_token(hasher, &field.signature.0, module);
            }
        },
        StructFieldInformation::DeclaredVariants(variants) => {
            hash_str(hasher, "variants");
            for variant in variants {
                let variant_name = module.identifier_at(variant.name);
                hash_str(hasher, variant_name.as_str());
                for field in &variant.fields {
                    let field_name = module.identifier_at(field.name);
                    hash_str(hasher, field_name.as_str());
                    hash_sig_token(hasher, &field.signature.0, module);
                }
            }
        },
    }
}

fn hash_function_def(hasher: &mut Sha256, fd: &FunctionDefinition, module: &CompiledModule) {
    let handle = module.function_handle_at(fd.function);
    let name = module.identifier_at(handle.name);
    hash_str(hasher, name.as_str());

    // Visibility
    hash_u8(hasher, fd.visibility as u8);

    // is_entry
    hash_bool(hasher, fd.is_entry);

    // Type parameter constraints
    for tp in &handle.type_parameters {
        hash_u8(hasher, tp.into_u8());
    }

    // Parameter types
    let params = module.signature_at(handle.parameters);
    for tok in &params.0 {
        hash_sig_token(hasher, tok, module);
    }

    // Return types
    let ret = module.signature_at(handle.return_);
    for tok in &ret.0 {
        hash_sig_token(hasher, tok, module);
    }
}

/// Serialize a `SignatureToken` into the hasher using stable canonical
/// names (e.g. `0x1::string::String`) instead of local bytecode indices.
/// This ensures the hash is independent of the internal index tables.
fn hash_sig_token(hasher: &mut Sha256, token: &SignatureToken, module: &CompiledModule) {
    let s = canonical_sig_token(token, module);
    hash_str(hasher, &s);
}

fn canonical_sig_token(token: &SignatureToken, module: &CompiledModule) -> String {
    match token {
        SignatureToken::Bool => "bool".to_owned(),
        SignatureToken::U8 => "u8".to_owned(),
        SignatureToken::U16 => "u16".to_owned(),
        SignatureToken::U32 => "u32".to_owned(),
        SignatureToken::U64 => "u64".to_owned(),
        SignatureToken::U128 => "u128".to_owned(),
        SignatureToken::U256 => "u256".to_owned(),
        SignatureToken::I8 => "i8".to_owned(),
        SignatureToken::I16 => "i16".to_owned(),
        SignatureToken::I32 => "i32".to_owned(),
        SignatureToken::I64 => "i64".to_owned(),
        SignatureToken::I128 => "i128".to_owned(),
        SignatureToken::I256 => "i256".to_owned(),
        SignatureToken::Address => "address".to_owned(),
        SignatureToken::Signer => "signer".to_owned(),
        SignatureToken::Vector(inner) => {
            format!("vector<{}>", canonical_sig_token(inner, module))
        },
        SignatureToken::Function(params, ret, abilities) => {
            let ps: Vec<_> = params
                .iter()
                .map(|t| canonical_sig_token(t, module))
                .collect();
            let rs: Vec<_> = ret.iter().map(|t| canonical_sig_token(t, module)).collect();
            format!(
                "|{}|{}[{}]",
                ps.join(","),
                rs.join(","),
                abilities.into_u8()
            )
        },
        SignatureToken::Struct(idx) => {
            let handle = module.struct_handle_at(*idx);
            let mod_handle = module.module_handle_at(handle.module);
            let addr = module.address_identifier_at(mod_handle.address);
            let mod_name = module.identifier_at(mod_handle.name);
            let struct_name = module.identifier_at(handle.name);
            format!("{}::{}::{}", addr.to_hex_literal(), mod_name, struct_name)
        },
        SignatureToken::StructInstantiation(idx, type_args) => {
            let base = canonical_sig_token(&SignatureToken::Struct(*idx), module);
            let args: Vec<_> = type_args
                .iter()
                .map(|t| canonical_sig_token(t, module))
                .collect();
            format!("{}<{}>", base, args.join(","))
        },
        SignatureToken::Reference(inner) => {
            format!("&{}", canonical_sig_token(inner, module))
        },
        SignatureToken::MutableReference(inner) => {
            format!("&mut {}", canonical_sig_token(inner, module))
        },
        SignatureToken::TypeParameter(idx) => format!("T{}", idx),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use move_binary_format::{
        file_format::{AddressIdentifierIndex, IdentifierIndex, ModuleHandle, ModuleHandleIndex},
        CompiledModule,
    };
    use move_core_types::{account_address::AccountAddress, identifier::Identifier};

    fn empty_module(addr: AccountAddress, name: &str) -> CompiledModule {
        let mut m = CompiledModule {
            version: move_binary_format::file_format_common::VERSION_MAX,
            ..Default::default()
        };
        m.address_identifiers.push(addr);
        m.identifiers.push(Identifier::new(name).unwrap());
        m.module_handles.push(ModuleHandle {
            address: AddressIdentifierIndex::new(0),
            name: IdentifierIndex::new(0),
        });
        m.self_module_handle_idx = ModuleHandleIndex::new(0);
        m
    }

    #[test]
    fn same_module_same_hash() {
        let m = empty_module(AccountAddress::ONE, "MyModule");
        let h1 = compute_interface_hash(&[&m]);
        let h2 = compute_interface_hash(&[&m]);
        assert_eq!(h1, h2, "same module must produce same hash");
    }

    #[test]
    fn different_module_name_different_hash() {
        let m1 = empty_module(AccountAddress::ONE, "Alpha");
        let m2 = empty_module(AccountAddress::ONE, "Beta");
        let h1 = compute_interface_hash(&[&m1]);
        let h2 = compute_interface_hash(&[&m2]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn module_order_does_not_matter() {
        let m1 = empty_module(AccountAddress::ONE, "A");
        let m2 = empty_module(AccountAddress::ONE, "B");
        let h1 = compute_interface_hash(&[&m1, &m2]);
        let h2 = compute_interface_hash(&[&m2, &m1]);
        assert_eq!(h1, h2, "module order must not affect hash");
    }
}
