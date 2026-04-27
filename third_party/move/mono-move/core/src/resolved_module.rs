// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Defines a wrapper for [`CompiledModule`] with all its type pre-interned.

use crate::{
    interner::{intern_sig_token, intern_struct_handle, Interner},
    types::{InternedType, EMPTY_TYPE_LIST},
};
use anyhow::{bail, Result};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        ConstantPoolIndex, FieldHandleIndex, SignatureIndex, StructDefinitionIndex,
        StructFieldInformation, StructHandleIndex, VariantFieldHandleIndex, VariantIndex,
    },
    CompiledModule,
};
use std::ops::Deref;

/// Wraps deserialized and verified [`CompiledModule`] with pre-interned type
/// pools. Users can use interned type representation directly using same table
/// indices, without any runtime interning.
pub struct ResolvedModule {
    /// Original compiled module. Kept so that its tables can be accessed.
    module: CompiledModule,
    /// Interned signatures from compiled module. Note that below tables are
    /// still needed because file format does not deduplicate all signatures.
    /// For example, fields carry their own signature and not an index into
    /// signatures pool.
    signatures: Vec<Vec<InternedType>>,
    /// Interned nominal types (structs or enums) from compiled module (both
    /// declared and imported).
    nominal_types: Vec<InternedType>,
    /// Interned struct or enum field types from compiled module. Only for
    /// declared structs or enums.
    field_types: Vec<FieldTypes>,
    /// Interned constant types from compiled module.
    constant_types: Vec<InternedType>,
}

/// Field types of any struct or enum definition in this module.
enum FieldTypes {
    Struct(Vec<InternedType>),
    Enum(Vec<Vec<InternedType>>),
}

// All compiled module pools are still accessible.
impl Deref for ResolvedModule {
    type Target = CompiledModule;

    fn deref(&self) -> &Self::Target {
        &self.module
    }
}

impl ResolvedModule {
    /// Returns interned types corresponding to the compiled module's
    /// signature.
    pub fn interned_types_at(&self, idx: SignatureIndex) -> &[InternedType] {
        &self.signatures[idx.0 as usize]
    }

    /// Returns interned type corresponding to the compiled module's nominal
    /// type (struct or enum). Note that this type can be imported from other
    /// module.
    pub fn interned_nominal_type_at(&self, idx: StructHandleIndex) -> InternedType {
        self.nominal_types[idx.0 as usize]
    }

    /// Returns interned type corresponding to the compiled module's nominal
    /// type (struct or enum) definition.
    pub fn interned_nominal_def_type_at(&self, def_idx: StructDefinitionIndex) -> InternedType {
        let idx = self.module.struct_def_at(def_idx).struct_handle;
        self.interned_nominal_type_at(idx)
    }

    /// Returns interned types corresponding to the compiled module's struct
    /// definition.
    pub fn interned_struct_field_types_at(
        &self,
        def_idx: StructDefinitionIndex,
    ) -> &[InternedType] {
        match &self.field_types[def_idx.0 as usize] {
            FieldTypes::Struct(field_types) => field_types.as_slice(),
            FieldTypes::Enum(..) => unreachable!("Must be a struct field"),
        }
    }

    /// Returns interned types corresponding to the compiled module's enum
    /// variant definition.
    pub fn interned_variant_field_types_at(
        &self,
        def_idx: StructDefinitionIndex,
        variant_idx: VariantIndex,
    ) -> &[InternedType] {
        match &self.field_types[def_idx.0 as usize] {
            FieldTypes::Enum(variants) => variants[variant_idx as usize].as_slice(),
            FieldTypes::Struct(..) => unreachable!("Must be an enum variant field"),
        }
    }

    /// Returns interned type corresponding to the compiled module's struct
    /// field.
    pub fn interned_field_type_at(&self, idx: FieldHandleIndex) -> InternedType {
        let h = self.module.field_handle_at(idx);
        self.interned_struct_field_types_at(h.owner)[h.field as usize]
    }

    /// Returns interned type corresponding to the compiled module's enum
    /// variant field.
    pub fn interned_variant_field_type_at(&self, idx: VariantFieldHandleIndex) -> InternedType {
        let h = self.module.variant_field_handle_at(idx);
        let fields = self.interned_variant_field_types_at(h.struct_index, h.variants[0]);
        fields[h.field as usize]
    }

    /// Returns interned type corresponding to the compiled module's constant.
    pub fn interned_constant_type_at(&self, idx: ConstantPoolIndex) -> InternedType {
        self.constant_types[idx.0 as usize]
    }

    /// Builds resolved module from compiled one, interning all signatures,
    /// field and constant types.
    pub fn build(module: CompiledModule, interner: &impl Interner) -> Result<Self> {
        let signatures = module
            .signatures()
            .iter()
            .map(|sig| {
                sig.0
                    .iter()
                    .map(|tok| intern_sig_token(tok, &module, interner))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;

        let nominal_types = module
            .struct_handles
            .iter()
            .map(|handle| {
                let (module_id, name) = intern_struct_handle(handle, &module, interner);
                let ty_args = if handle.type_parameters.is_empty() {
                    EMPTY_TYPE_LIST
                } else {
                    let params = (0..handle.type_parameters.len())
                        .map(|idx| interner.type_param_of(idx as u16))
                        .collect::<Vec<_>>();
                    interner.type_list_of(&params)
                };
                interner.nominal_of(module_id, name, ty_args)
            })
            .collect();

        let field_types = module
            .struct_defs()
            .iter()
            .map(|def| {
                Ok(match &def.field_information {
                    StructFieldInformation::Native => {
                        bail!("Native fields are deprecated");
                    },
                    StructFieldInformation::Declared(fields) => {
                        let fields = fields
                            .iter()
                            .map(|f| intern_sig_token(&f.signature.0, &module, interner))
                            .collect::<Result<Vec<_>>>()?;
                        FieldTypes::Struct(fields)
                    },
                    StructFieldInformation::DeclaredVariants(variants) => {
                        let variants = variants
                            .iter()
                            .map(|v| {
                                v.fields
                                    .iter()
                                    .map(|f| intern_sig_token(&f.signature.0, &module, interner))
                                    .collect::<Result<Vec<_>>>()
                            })
                            .collect::<Result<Vec<_>>>()?;
                        FieldTypes::Enum(variants)
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let constant_types = module
            .constant_pool()
            .iter()
            .map(|c| intern_sig_token(&c.type_, &module, interner))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            module,
            signatures,
            nominal_types,
            field_types,
            constant_types,
        })
    }
}
