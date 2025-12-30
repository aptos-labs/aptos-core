// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    file_format::{
        empty_module, Bytecode, CodeUnit, FieldDefinition, FunctionAttribute, FunctionDefinition,
        FunctionHandle, IdentifierIndex, ModuleHandleIndex, Signature, SignatureIndex,
        SignatureToken, StructDefinition, StructDefinitionIndex, StructFieldInformation,
        StructHandle, StructHandleIndex, StructVariantHandle, TypeSignature, VariantDefinition,
        VariantFieldHandle, VariantFieldHandleIndex, VariantFieldInstantiation,
    },
    CompiledModule,
};
use move_core_types::{ability::AbilitySet, identifier::Identifier};

/// Create a dummy module to wrap the bytecode program in local@code
pub fn dummy_procedure_module(code: Vec<Bytecode>) -> CompiledModule {
    let mut module = empty_module();
    let code_unit = CodeUnit {
        code,
        ..Default::default()
    };
    let fun_def = FunctionDefinition {
        code: Some(code_unit),
        ..Default::default()
    };

    let fun_handle = FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(0),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![],
    };

    module.function_handles.push(fun_handle);
    module.function_defs.push(fun_def);
    module
}

/// Create a module with a struct
/// ```text
/// module <SELF> {
///     struct Test {
///         field: bool
///     }
/// }
/// ```
pub fn module_with_struct() -> CompiledModule {
    let mut module = empty_module();
    let name = Identifier::new("Test".to_string()).unwrap();
    module.identifiers.push(name);
    let field_name = Identifier::new("field".to_string()).unwrap();
    module.identifiers.push(field_name);

    let struct_handle = StructHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(1),
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![],
    };
    module.struct_handles.push(struct_handle);

    // struct signature: 1
    let signature_token = SignatureToken::Struct(StructHandleIndex(0));
    let signature = Signature(vec![signature_token]);
    module.signatures.push(signature);

    // bool signature: 2
    let bool_token = SignatureToken::Bool;
    let bool_signature = Signature(vec![bool_token.clone()]);
    module.signatures.push(bool_signature);

    let field = FieldDefinition {
        name: IdentifierIndex(2),
        signature: TypeSignature(bool_token),
    };

    let struct_def = StructDefinition {
        struct_handle: StructHandleIndex(0),
        field_information: StructFieldInformation::Declared(vec![field]),
    };
    module.struct_defs.push(struct_def);

    module
}

/// Create a module with a enum
/// ```text
/// module <SELF> {
///     enum Test {
///         A {
///             field: bool
///         }
///         B {
///             field: bool
///         }
///     }
/// }
/// ```
pub fn module_with_enum() -> CompiledModule {
    let mut module = empty_module();

    let name = Identifier::new("Test".to_string()).unwrap();
    // name index: 1
    module.identifiers.push(name);
    // name index: 2
    let variant_name_a = Identifier::new("A".to_string()).unwrap();
    module.identifiers.push(variant_name_a);
    // name index: 3
    let variant_name_b = Identifier::new("B".to_string()).unwrap();
    module.identifiers.push(variant_name_b);
    // name index: 4
    let field_name = Identifier::new("field".to_string()).unwrap();
    module.identifiers.push(field_name);

    let struct_handle = StructHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(1),
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![],
    };
    module.struct_handles.push(struct_handle);

    let struct_variant_handle = StructVariantHandle {
        struct_index: StructDefinitionIndex(0),
        variant: 0,
    };
    module.struct_variant_handles.push(struct_variant_handle);

    // struct signature: 1
    let signature_token = SignatureToken::Struct(StructHandleIndex(0));
    let signature = Signature(vec![signature_token]);
    module.signatures.push(signature);

    // bool signature: 2
    let bool_token = SignatureToken::Bool;
    let bool_signature = Signature(vec![bool_token.clone()]);
    module.signatures.push(bool_signature);

    let variant_a = VariantDefinition {
        name: IdentifierIndex(2),
        fields: vec![FieldDefinition {
            name: IdentifierIndex(4),
            signature: TypeSignature(bool_token.clone()),
        }],
    };

    let variant_b = VariantDefinition {
        name: IdentifierIndex(3),
        fields: vec![FieldDefinition {
            name: IdentifierIndex(4),
            signature: TypeSignature(bool_token),
        }],
    };

    let variant_field = VariantFieldHandle {
        struct_index: StructDefinitionIndex(0),
        variants: vec![0, 1],
        field: 0,
    };
    module.variant_field_handles.push(variant_field);
    let variant_field_instantiation = VariantFieldInstantiation {
        handle: VariantFieldHandleIndex(0),
        type_parameters: SignatureIndex(0),
    };
    module
        .variant_field_instantiations
        .push(variant_field_instantiation);

    let struct_def = StructDefinition {
        struct_handle: StructHandleIndex(0),
        field_information: StructFieldInformation::DeclaredVariants(vec![variant_a, variant_b]),
    };
    module.struct_defs.push(struct_def);

    module
}

pub fn module_with_struct_api_attributes(
    module: &mut CompiledModule,
    code: Vec<Bytecode>,
    attributes: Vec<FunctionAttribute>,
    para_sig: Signature,
    return_sig: Signature,
) {
    let code_unit = CodeUnit {
        code,
        ..Default::default()
    };
    let fun_def = FunctionDefinition {
        code: Some(code_unit),
        ..Default::default()
    };
    module.signatures.push(para_sig);
    module.signatures.push(return_sig);

    let fun_handle = FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(0),
        parameters: SignatureIndex(3),
        return_: SignatureIndex(4),
        type_parameters: vec![],
        access_specifiers: None,
        attributes,
    };

    module.function_handles.push(fun_handle);
    module.function_defs.push(fun_def);
}
