// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(unused)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use anyhow::{bail, Error, Result};
use clap::Parser;
use move_binary_format::{
    binary_views::BinaryIndexedView,
    file_format::{
        Bytecode, CodeUnit, FunctionDefinition, FunctionDefinitionIndex, FunctionHandle, Signature,
        SignatureIndex, StructDefinition, StructDefinitionIndex, StructFieldInformation,
        TableIndex,
    },
};
use move_bytecode_source_map::{mapping::SourceMapping, source_map::FunctionSourceMap};

use move_core_types::identifier::IdentStr;

use llvm_sys::{
    core::{
        LLVMAddFunction, LLVMAddGlobal, LLVMDisposeMessage, LLVMDumpModule, LLVMFunctionType,
        LLVMGetStructName, LLVMModuleCreateWithNameInContext,
    },
    prelude::{LLVMContextRef as LLVMContext, LLVMModuleRef, LLVMTypeRef, LLVMValueRef},
    target_machine::LLVMCodeGenOptLevel,
};

use move_ir_types::location::Loc;

use std::{ffi::CStr, fs::File, ptr};

use crate::{move_bpf_module::MoveBPFModule, support::to_c_str};

/// Holds the various options that we support while disassembling code.
#[derive(Debug, Default, Parser)]
pub struct DisassemblerOptions {
    /// Only print non-private functions
    #[clap(long = "only-public")]
    pub only_externally_visible: bool,

    /// Print the bytecode for the instructions within the function.
    #[clap(long = "print-code")]
    pub print_code: bool,

    /// Print the basic blocks of the bytecode.
    #[clap(long = "print-basic-blocks")]
    pub print_basic_blocks: bool,

    /// Print the locals inside each function body.
    #[clap(long = "print-locals")]
    pub print_locals: bool,
}

impl DisassemblerOptions {
    pub fn new() -> Self {
        Self {
            only_externally_visible: false,
            print_code: true,
            print_basic_blocks: true,
            print_locals: true,
        }
    }
}

pub struct Disassembler<'a> {
    source_mapper: SourceMapping<'a>,
    llvm_context: LLVMContext,
}

impl<'a> Disassembler<'a> {
    pub fn new(source_mapper: SourceMapping<'a>, llvm_context: LLVMContext) -> Self {
        Self {
            source_mapper,
            llvm_context,
        }
    }

    //***************************************************************************
    // Helpers
    //***************************************************************************

    fn get_function_def(
        &self,
        function_definition_index: FunctionDefinitionIndex,
    ) -> Result<&FunctionDefinition> {
        if function_definition_index.0 as usize
            >= self
                .source_mapper
                .bytecode
                .function_defs()
                .map_or(0, |f| f.len())
        {
            bail!("Invalid function definition index supplied when marking function")
        }
        match self
            .source_mapper
            .bytecode
            .function_def_at(function_definition_index)
        {
            Ok(definition) => Ok(definition),
            Err(err) => Err(Error::new(err)),
        }
    }

    fn get_struct_def(
        &self,
        struct_definition_index: StructDefinitionIndex,
    ) -> Result<&StructDefinition> {
        if struct_definition_index.0 as usize
            >= self
                .source_mapper
                .bytecode
                .struct_defs()
                .map_or(0, |d| d.len())
        {
            bail!("Invalid struct definition index supplied when marking struct")
        }
        match self
            .source_mapper
            .bytecode
            .struct_def_at(struct_definition_index)
        {
            Ok(definition) => Ok(definition),
            Err(err) => Err(Error::new(err)),
        }
    }

    pub fn process_function_def(
        &self,
        function_source_map: &FunctionSourceMap,
        function: Option<(&FunctionDefinition, &FunctionHandle)>,
        name: &IdentStr,
        parameters: SignatureIndex,
        code: Option<&CodeUnit>,
        move_module: &mut MoveBPFModule,
    ) -> LLVMValueRef {
        let parameter_list = &self.source_mapper.bytecode.signature_at(parameters).0;

        let ret_type = match function {
            Some(function) => self
                .source_mapper
                .bytecode
                .signature_at(function.1.return_)
                .0
                .clone(),
            None => vec![],
        };
        let ts = move_module.llvm_type_for_sig_tokens(&ret_type);
        let llvm_type_return = move_module.llvm_make_single_return_type(ts);

        let mut llvm_type_parameters =
            move_module.llvm_type_for_sig_tokens(&parameter_list.to_vec());

        let fn_value = unsafe {
            LLVMAddFunction(
                move_module.module,
                to_c_str(name.as_str()).as_ptr(),
                LLVMFunctionType(
                    llvm_type_return,
                    llvm_type_parameters.as_mut_ptr(),
                    llvm_type_parameters.len() as u32,
                    false as i32,
                ),
            )
        };

        let entry_block = move_module.append_basic_block(fn_value, "entry");

        // Iterate over all the bytecode instructions and generate llvm-ir.
        let _bytecode = self.disassemble_bytecode(
            function_source_map,
            name,
            parameters,
            code.unwrap(),
            move_module,
        );

        move_module.position_at_end(entry_block);
        move_module.build_return(move_module.llvm_constant(0));

        verify_function(fn_value);

        fn_value
    }

    pub fn process_struct_def(
        &self,
        struct_def_idx: StructDefinitionIndex,
        move_module: &mut MoveBPFModule,
    ) -> Result<LLVMTypeRef> {
        let struct_def = self.get_struct_def(struct_def_idx)?;
        let llvm_struct = move_module.llvm_struct_from_index(&struct_def.struct_handle);
        let mut llvm_elem_types: Vec<LLVMTypeRef> = Vec::new();
        match &struct_def.field_information {
            StructFieldInformation::Native => return Ok(llvm_struct),
            StructFieldInformation::Declared(fields) => Some(for field_definition in fields {
                let x = move_module.llvm_type_for_sig_tok(&field_definition.signature.0);
                llvm_elem_types.push(x);
            }),
        };
        move_module.llvm_set_struct_body(llvm_struct, &mut llvm_elem_types);
        let name = unsafe { LLVMGetStructName(llvm_struct) };
        if !name.is_null() {
            unsafe { LLVMAddGlobal(move_module.module, llvm_struct, name) };
        };
        Ok(llvm_struct)
    }

    fn disassemble_instruction(
        &self,
        parameters: &Signature,
        instruction: &Bytecode,
        locals_sigs: &Signature,
        function_source_map: &FunctionSourceMap,
        default_location: &Loc,
    ) -> Result<String> {
        Ok("Ok".to_string())
    }

    pub fn disassemble_bytecode(
        &self,
        function_source_map: &FunctionSourceMap,
        function_name: &IdentStr,
        parameters: SignatureIndex,
        code: &CodeUnit,
        move_module: &mut MoveBPFModule,
    ) -> Result<Vec<String>> {
        let parameters = self.source_mapper.bytecode.signature_at(parameters);
        let locals_sigs = self.source_mapper.bytecode.signature_at(code.locals);

        // let function_code_coverage_map = self.get_function_coverage(function_name);

        let decl_location = &function_source_map.definition_location;

        // TODO: Construct the instructions in module directly.
        let instrs = code
            .code
            .iter()
            .map(|instruction| {
                self.disassemble_instruction(
                    parameters,
                    instruction,
                    locals_sigs,
                    function_source_map,
                    decl_location,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(vec!["".to_string()])
    }

    pub fn disassemble(&mut self) -> Result<LLVMModuleRef> {
        let name_opt = self.source_mapper.source_map.module_name_opt.as_ref();
        let name = name_opt.map(|(addr, n)| format!("{}.{}", addr.short_str_lossless(), n));
        let llvm_module_name: String;
        let header = match name {
            Some(s) => {
                llvm_module_name = String::clone(&s) + ".bc";
                format!("module {}", s)
            }
            None => {
                llvm_module_name = "script.bc".to_string();
                "script".to_owned()
            }
        };

        let c_string = to_c_str(&header);

        let opt = LLVMCodeGenOptLevel::LLVMCodeGenLevelNone; // TODO: Add optimization based on command line flag.
        let mut move_module = MoveBPFModule::new(
            &self.llvm_context,
            &header,
            &llvm_module_name,
            opt,
            &self.source_mapper,
        );

        for i in 0..self
            .source_mapper
            .bytecode
            .struct_defs()
            .map_or(0, |d| d.len())
        {
            self.process_struct_def(StructDefinitionIndex(i as TableIndex), &mut move_module)?;
        }

        match self.source_mapper.bytecode {
            BinaryIndexedView::Script(script) => {
                self.process_function_def(
                    self.source_mapper
                        .source_map
                        .get_function_source_map(FunctionDefinitionIndex(0_u16))?,
                    None,
                    IdentStr::new("main")?,
                    script.parameters,
                    Some(&script.code),
                    &mut move_module,
                );
            }
            BinaryIndexedView::Module(module) => {
                for i in 0..module.function_defs.len() {
                    let function_definition_index = FunctionDefinitionIndex(i as TableIndex);
                    let function_definition = self.get_function_def(function_definition_index)?;
                    let function_handle = self
                        .source_mapper
                        .bytecode
                        .function_handle_at(function_definition.function);
                    self.process_function_def(
                        self.source_mapper
                            .source_map
                            .get_function_source_map(function_definition_index)?,
                        Some((function_definition, function_handle)),
                        self.source_mapper
                            .bytecode
                            .identifier_at(function_handle.name),
                        function_handle.parameters,
                        function_definition.code.as_ref(),
                        &mut move_module,
                    );
                }
            }
        };

        verify_module(move_module.module);

        Ok(move_module.module)
    }

    pub fn llvm_write_to_file(
        &self,
        module: LLVMModuleRef,
        llvm_ir: bool,
        output_file_name: &String,
    ) -> Result<()> {
        use llvm_sys::{
            bit_writer::LLVMWriteBitcodeToFD,
            core::{LLVMPrintModuleToFile, LLVMPrintModuleToString},
        };
        use std::os::unix::io::AsRawFd;

        unsafe {
            if llvm_ir {
                if output_file_name != "-" {
                    let mut err_string = ptr::null_mut();
                    let res = LLVMPrintModuleToFile(
                        module,
                        to_c_str(output_file_name).as_ptr(),
                        &mut err_string,
                    );

                    if res != 0 {
                        assert!(!err_string.is_null());
                        let msg = CStr::from_ptr(err_string).to_string_lossy();
                        LLVMDisposeMessage(err_string);
                        anyhow::bail!("{}", msg);
                    }
                } else {
                    let buf = LLVMPrintModuleToString(module);
                    assert!(!buf.is_null());
                    let cstr = CStr::from_ptr(buf);
                    print!("{}", cstr.to_string_lossy());
                    LLVMDisposeMessage(buf);
                }
            } else {
                if output_file_name == "-" {
                    anyhow::bail!("Not writing bitcode to stdout");
                }
                let bc_file = File::create(output_file_name)?;
                let res =
                    LLVMWriteBitcodeToFD(module, bc_file.as_raw_fd(), false as i32, true as i32);

                if res != 0 {
                    anyhow::bail!("Failed to write bitcode to file");
                }
            }
        }

        Ok(())
    }
}

fn verify_function(llfn: LLVMValueRef) {
    use llvm_sys::analysis::*;
    unsafe {
        LLVMVerifyFunction(llfn, LLVMVerifierFailureAction::LLVMAbortProcessAction);
    }
}

fn verify_module(llmod: LLVMModuleRef) {
    use llvm_sys::analysis::*;
    unsafe {
        LLVMVerifyModule(
            llmod,
            LLVMVerifierFailureAction::LLVMAbortProcessAction,
            ptr::null_mut(),
        );
    }
}
