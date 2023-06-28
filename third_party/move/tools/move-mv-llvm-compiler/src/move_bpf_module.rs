// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use llvm_sys::core::{
    LLVMAddModuleFlag, LLVMAppendBasicBlockInContext, LLVMBuildRet, LLVMBuildRetVoid, LLVMConstInt,
    LLVMCreateBuilderInContext, LLVMGetBasicBlockParent, LLVMGetNextBasicBlock, LLVMGetTypeKind,
    LLVMInsertBasicBlockInContext, LLVMInt128TypeInContext, LLVMInt1TypeInContext,
    LLVMInt32TypeInContext, LLVMInt64TypeInContext, LLVMInt8TypeInContext, LLVMIsOpaqueStruct,
    LLVMModuleCreateWithNameInContext, LLVMPointerType, LLVMPositionBuilderAtEnd, LLVMSetTarget,
    LLVMStructCreateNamed, LLVMStructSetBody, LLVMStructTypeInContext, LLVMTypeOf, LLVMVoidType,
};

use llvm_sys::{
    debuginfo::{LLVMCreateDIBuilder, LLVMDIBuilderCreateFile},
    prelude::{
        LLVMBasicBlockRef, LLVMBuilderRef, LLVMContextRef, LLVMDIBuilderRef, LLVMMetadataRef,
        LLVMModuleRef, LLVMTypeRef, LLVMValueRef,
    },
    target_machine::{
        LLVMCodeGenOptLevel, LLVMCodeModel, LLVMCreateTargetMachine, LLVMGetTargetFromName,
        LLVMRelocMode, LLVMTargetMachineRef, LLVMTargetRef,
    },
    LLVMModuleFlagBehavior, LLVMTypeKind,
};

use crate::support::{to_c_str, LLVMString};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    ffi::CStr,
    fmt::{self, Debug},
    marker::PhantomData,
};

use move_binary_format::file_format::{SignatureToken, StructHandleIndex, TypeParameterIndex};
use move_bytecode_source_map::mapping::SourceMapping;
use once_cell::sync::OnceCell;

static LLVM_INIT: OnceCell<()> = OnceCell::new();

/// Source file scope for debug info
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DIFile<'ctx> {
    pub(crate) metadata_ref: LLVMMetadataRef,
    _marker: PhantomData<&'ctx LLVMContextRef>,
}

/// Compilation unit scope for debug info
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DICompileUnit<'ctx> {
    file: DIFile<'ctx>,
    pub(crate) metadata_ref: LLVMMetadataRef,
    _marker: PhantomData<&'ctx LLVMContextRef>,
}

impl<'ctx> DICompileUnit<'ctx> {
    pub fn get_file(&self) -> DIFile<'ctx> {
        self.file
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InitializationConfig {
    pub asm_parser: bool,
    pub asm_printer: bool,
    pub base: bool,
    pub disassembler: bool,
    pub info: bool,
    pub machine_code: bool,
}

impl Default for InitializationConfig {
    fn default() -> Self {
        InitializationConfig {
            asm_parser: true,
            asm_printer: true,
            base: true,
            disassembler: true,
            info: true,
            machine_code: true,
        }
    }
}

static TARGET_LOCK: Lazy<RwLock<()>> = Lazy::new(|| RwLock::new(()));

#[derive(Eq)]
pub struct TargetTriple {
    pub(crate) triple: LLVMString,
}

impl TargetTriple {
    pub fn create(triple: &str) -> TargetTriple {
        let c_string = to_c_str(triple);

        TargetTriple {
            triple: LLVMString::create_from_c_str(&c_string),
        }
    }

    pub fn as_str(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.as_ptr()) }
    }

    pub fn as_ptr(&self) -> *const ::libc::c_char {
        self.triple.as_ptr()
    }
}

impl PartialEq for TargetTriple {
    fn eq(&self, other: &TargetTriple) -> bool {
        self.triple == other.triple
    }
}

impl fmt::Debug for TargetTriple {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TargetTriple({:?})", self.triple)
    }
}

impl fmt::Display for TargetTriple {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "TargetTriple({:?})", self.triple)
    }
}

pub struct MoveBPFModule<'a> {
    pub name: String,
    pub module: LLVMModuleRef, // Some things in inkwell are good. like Module which takes lifetime parameters. That might help getting around borrow checker issues down the road.
    pub builder: LLVMBuilderRef,
    pub dibuilder: LLVMDIBuilderRef,
    pub di_compile_unit: DICompileUnit<'a>,
    pub context: &'a LLVMContextRef,
    pub opt: LLVMCodeGenOptLevel,
    pub source_mapper: &'a SourceMapping<'a>,
    pub struct_mapper: HashMap<i32, LLVMTypeRef>,
    pub address_type: LLVMTypeRef,
    pub signer_type: LLVMTypeRef,
    pub type_param_mapper: HashMap<i32, LLVMTypeRef>,
}

impl<'a> MoveBPFModule<'a> {
    fn llvm_target_triple() -> TargetTriple {
        TargetTriple::create("bpfel-unknown-unknown")
    }

    fn llvm_target_name() -> &'static str {
        "bpfel" // bpf little endian.
    }

    fn llvm_features() -> &'static str {
        "" // no additional target specific features.
    }

    pub fn initialize_bpf(config: &InitializationConfig) {
        use llvm_sys::target::{
            LLVMInitializeBPFAsmPrinter, LLVMInitializeBPFTarget, LLVMInitializeBPFTargetInfo,
            LLVMInitializeBPFTargetMC,
        };

        if config.base {
            let _guard = TARGET_LOCK.write();
            unsafe { LLVMInitializeBPFTarget() };
        }

        if config.info {
            let _guard = TARGET_LOCK.write();
            unsafe { LLVMInitializeBPFTargetInfo() };
        }

        if config.asm_printer {
            let _guard = TARGET_LOCK.write();
            unsafe { LLVMInitializeBPFAsmPrinter() };
        }

        // No asm parser

        if config.disassembler {
            use llvm_sys::target::LLVMInitializeBPFDisassembler;

            let _guard = TARGET_LOCK.write();
            unsafe { LLVMInitializeBPFDisassembler() };
        }

        if config.machine_code {
            let _guard = TARGET_LOCK.write();
            unsafe { LLVMInitializeBPFTargetMC() };
        }
    }

    pub fn get_target_machine(&self) -> Option<LLVMTargetMachineRef> {
        Self::initialize_bpf(&InitializationConfig::default());

        let opt_level = LLVMCodeGenOptLevel::LLVMCodeGenLevelNone; // TODO: Add optimization based on command line flag.
        let reloc_mode = LLVMRelocMode::LLVMRelocDefault;
        let code_model = LLVMCodeModel::LLVMCodeModelDefault;
        let llvm_target_name_ptr = to_c_str(Self::llvm_target_name()).as_ptr();
        let target: LLVMTargetRef = unsafe { LLVMGetTargetFromName(llvm_target_name_ptr) };
        let cpu = "v2";

        let target_machine = unsafe {
            LLVMCreateTargetMachine(
                target,
                MoveBPFModule::llvm_target_triple().as_ptr(),
                to_c_str(cpu).as_ptr(),
                to_c_str(MoveBPFModule::llvm_features()).as_ptr(),
                opt_level,
                reloc_mode,
                code_model,
            )
        };

        assert!(!target_machine.is_null());
        Some(target_machine)
    }

    pub fn add_basic_value_flag(
        module: LLVMModuleRef,
        key: &str,
        behavior: LLVMModuleFlagBehavior,
        flag: LLVMValueRef,
    ) {
        use llvm_sys::core::LLVMValueAsMetadata;

        let md = unsafe { LLVMValueAsMetadata(flag) };

        unsafe {
            LLVMAddModuleFlag(
                module,
                behavior,
                key.as_ptr() as *mut ::libc::c_char,
                key.len(),
                md,
            )
        }
    }

    pub fn set_source_file_name(module: LLVMModuleRef, file_name: &str) {
        use llvm_sys::core::LLVMSetSourceFileName;

        unsafe {
            LLVMSetSourceFileName(
                module,
                file_name.as_ptr() as *const ::libc::c_char,
                file_name.len(),
            )
        }
    }

    pub fn new(
        context: &'a LLVMContextRef,
        name: &str,
        filename: &str,
        opt: LLVMCodeGenOptLevel,
        source_mapper: &'a SourceMapping,
    ) -> Self {
        LLVM_INIT.get_or_init(|| {
            Self::initialize_bpf(&InitializationConfig::default());
        });

        let triple = MoveBPFModule::llvm_target_triple();
        let c_string = to_c_str(name);

        let module = unsafe { LLVMModuleCreateWithNameInContext(c_string.as_ptr(), *context) };

        let debug_metadata_version =
            unsafe { LLVMConstInt(LLVMInt64TypeInContext(*context), 3, false as i32) };
        Self::add_basic_value_flag(
            module,
            "Debug Info Version",
            LLVMModuleFlagBehavior::LLVMModuleFlagBehaviorWarning,
            debug_metadata_version,
        );

        let builder = unsafe { LLVMCreateBuilderInContext(*context) };

        let dibuilder = unsafe { LLVMCreateDIBuilder(module) };

        //let dibuilder = DebugInfoBuilder {
        //    builder,
        //    _marker: PhantomData,
        //};

        let directory = ".";
        //let file = builder.create_file(filename, directory);

        let file_metadata_ref = unsafe {
            LLVMDIBuilderCreateFile(
                dibuilder,
                filename.as_ptr() as _,
                filename.len(),
                directory.as_ptr() as _,
                directory.len(),
            )
        };

        let di_compile_unit = DICompileUnit {
            file: DIFile {
                metadata_ref: file_metadata_ref,
                _marker: PhantomData,
            },
            metadata_ref: file_metadata_ref,
            _marker: PhantomData,
        };

        unsafe { LLVMSetTarget(module, triple.as_ptr()) }
        Self::set_source_file_name(module, filename);

        let struct_mapper: HashMap<i32, LLVMTypeRef> = HashMap::new();
        let type_param_mapper: HashMap<i32, LLVMTypeRef> = HashMap::new();

        let address_type = unsafe { LLVMStructCreateNamed(*context, to_c_str("address").as_ptr()) };
        let signer_type = unsafe { LLVMStructCreateNamed(*context, to_c_str("signer").as_ptr()) };

        MoveBPFModule {
            name: name.to_owned(),
            module,
            builder,
            dibuilder,
            di_compile_unit,
            context,
            opt,
            source_mapper,
            struct_mapper,
            address_type,
            signer_type,
            type_param_mapper,
        }
    }

    pub fn llvm_type_for_sig_tok(&mut self, sig_tok: &SignatureToken) -> LLVMTypeRef {
        match sig_tok {
            // TODO: Use llvm::Context types
            SignatureToken::Bool => unsafe { LLVMInt1TypeInContext(*self.context) },
            SignatureToken::U8 => unsafe { LLVMInt8TypeInContext(*self.context) },
            SignatureToken::U32 => unsafe { LLVMInt32TypeInContext(*self.context) },
            SignatureToken::U64 => unsafe { LLVMInt64TypeInContext(*self.context) },
            SignatureToken::U128 => unsafe { LLVMInt128TypeInContext(*self.context) },
            SignatureToken::Struct(idx) => self.llvm_struct_from_index(idx),
            SignatureToken::Address => self.address_type,
            SignatureToken::Signer => self.signer_type,
            SignatureToken::Reference(inner) => unsafe {
                LLVMPointerType(self.llvm_type_for_sig_tok(inner), 0)
            },
            SignatureToken::StructInstantiation(idx, type_arguments) => {
                self.llvm_struct_from_instance(idx, type_arguments)
            }
            SignatureToken::TypeParameter(idx) => self.llvm_type_parameter_from_index(idx),
            _ => unimplemented!("Remaining Signature tokens to be implemented"),
        }
    }
    pub fn llvm_type_for_sig_tokens(
        &mut self,
        sig_tokens: &Vec<SignatureToken>,
    ) -> Vec<LLVMTypeRef> {
        let mut vec = Vec::new();
        for v in sig_tokens {
            vec.push(self.llvm_type_for_sig_tok(v));
        }
        vec
    }
    pub fn llvm_make_single_return_type(&mut self, mut types: Vec<LLVMTypeRef>) -> LLVMTypeRef {
        if types.is_empty() {
            unsafe { LLVMVoidType() }
        } else if types.len() == 1 {
            types[0]
        } else {
            unsafe {
                LLVMStructTypeInContext(
                    *self.context,
                    types[..].as_mut_ptr(),
                    types.len() as u32,
                    false as i32,
                )
            }
        }
    }
    pub fn llvm_constant(&self, value: u64) -> LLVMValueRef {
        // TODO: Return a constant value corresponding to the input type.
        unsafe { LLVMConstInt(LLVMInt64TypeInContext(*self.context), value, false as i32) }
    }

    pub fn get_next_basic_block(
        &self,
        basic_block: LLVMBasicBlockRef,
    ) -> Option<LLVMBasicBlockRef> {
        let next_bb = unsafe { LLVMGetNextBasicBlock(basic_block) };
        if next_bb.is_null() {
            return None;
        }
        Some(next_bb)
    }

    pub fn append_basic_block(&self, function: LLVMValueRef, name: &str) -> LLVMBasicBlockRef {
        let c_string = to_c_str(name);
        unsafe { LLVMAppendBasicBlockInContext(*self.context, function, c_string.as_ptr()) }
    }

    pub fn prepend_basic_block(
        &self,
        basic_block: LLVMBasicBlockRef,
        name: &str,
    ) -> LLVMBasicBlockRef {
        let c_string = to_c_str(name);
        unsafe { LLVMInsertBasicBlockInContext(*self.context, basic_block, c_string.as_ptr()) }
    }

    pub fn insert_basic_block_after(
        &self,
        basic_block: LLVMBasicBlockRef,
        name: &str,
    ) -> LLVMBasicBlockRef {
        //let next_basic_block = &self.get_next_basic_block(basic_block);
        match self.get_next_basic_block(basic_block) {
            Some(bb) => self.prepend_basic_block(bb, name),
            None => unsafe { self.append_basic_block(LLVMGetBasicBlockParent(basic_block), name) },
        }
    }

    pub fn position_at_end(&self, basic_block: LLVMBasicBlockRef) {
        unsafe {
            LLVMPositionBuilderAtEnd(self.builder, basic_block);
        }
    }
    pub fn build_return(&self, value: LLVMValueRef) {
        unsafe {
            match LLVMGetTypeKind(LLVMTypeOf(value)) {
                LLVMTypeKind::LLVMVoidTypeKind => LLVMBuildRetVoid(self.builder),
                _ => LLVMBuildRet(self.builder, value),
            }
        };
    }
    pub fn llvm_struct_from_index(&mut self, struct_handle_idx: &StructHandleIndex) -> LLVMTypeRef {
        let index = struct_handle_idx.0 as i32;
        if let Some(x) = self.struct_mapper.get(&index) {
            return *x;
        }
        let struct_handle = self
            .source_mapper
            .bytecode
            .struct_handle_at(*struct_handle_idx);
        let name = self
            .source_mapper
            .bytecode
            .identifier_at(struct_handle.name)
            .to_string();
        let name2 = name.as_str();
        let s = unsafe { LLVMStructCreateNamed(*self.context, to_c_str(name2).as_ptr()) };
        self.struct_mapper.insert(index, s);
        s
    }

    pub fn llvm_set_struct_body(
        &self,
        struct_type: LLVMTypeRef,
        elem_types: &mut Vec<LLVMTypeRef>,
    ) {
        if unsafe { LLVMIsOpaqueStruct(struct_type) } != 0 {
            unsafe {
                LLVMStructSetBody(
                    struct_type,
                    elem_types[..].as_mut_ptr(),
                    elem_types.len() as u32,
                    false as i32,
                )
            };
        }
    }

    pub fn llvm_struct_from_instance(
        &mut self,
        struct_handle_idx: &StructHandleIndex,
        elem_types: &Vec<SignatureToken>,
    ) -> LLVMTypeRef {
        let index = struct_handle_idx.0 as i32;
        if let Some(x) = self.struct_mapper.get(&index) {
            return *x;
        }
        let mut v = self.llvm_type_for_sig_tokens(elem_types);
        let s = unsafe {
            LLVMStructTypeInContext(
                *self.context,
                v[..].as_mut_ptr(),
                v.len() as u32,
                false as i32,
            )
        };
        self.struct_mapper.insert(index, s);
        s
    }

    pub fn llvm_type_parameter_from_index(
        &mut self,
        type_param_idx: &TypeParameterIndex,
    ) -> LLVMTypeRef {
        let index = *type_param_idx as i32;
        if let Some(x) = self.type_param_mapper.get(&index) {
            return *x;
        }
        let name = format!("type_param_{}", index);
        let s = unsafe { LLVMStructCreateNamed(*self.context, to_c_str(name.as_str()).as_ptr()) };
        self.type_param_mapper.insert(index, s);
        s
    }
}
