// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLVM wrappers.
//!
//! The stackless code generator accesses llvm only through this mod.
//!
//! It:
//!
//! - Runs dtors
//! - Encapsulates unsafety, though making LLVM fully memsafe is hard.
//! - Hides weirdly mutable array pointers.
//! - Provides high-level instruction builders compatible with the stackless bytecode model.

use llvm_extra_sys::*;
use llvm_sys::{core::*, prelude::*, target::*, target_machine::*, LLVMOpcode, LLVMUnnamedAddr};
use move_core_types::u256;
use num_traits::{PrimInt, ToPrimitive};

use crate::cstr::SafeCStr;

use std::{
    ffi::{CStr, CString},
    ptr,
};

pub use llvm_extra_sys::AttributeKind;
pub use llvm_sys::{
    LLVMIntPredicate, LLVMLinkage, LLVMLinkage::LLVMInternalLinkage,
    LLVMTypeKind::LLVMIntegerTypeKind,
};

pub fn initialize_sbf() {
    unsafe {
        LLVMInitializeSBFTargetInfo();
        LLVMInitializeSBFTarget();
        LLVMInitializeSBFTargetMC();
        LLVMInitializeSBFAsmPrinter();
        LLVMInitializeSBFAsmParser();
    }
}

pub struct Context(LLVMContextRef);

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            LLVMContextDispose(self.0);
        }
    }
}

impl Context {
    pub fn new() -> Context {
        unsafe { Context(LLVMContextCreate()) }
    }

    pub fn create_module(&self, name: &str) -> Module {
        unsafe { Module(LLVMModuleCreateWithNameInContext(name.cstr(), self.0)) }
    }

    pub fn create_builder(&self) -> Builder {
        unsafe { Builder(LLVMCreateBuilderInContext(self.0)) }
    }

    pub fn get_anonymous_struct_type(&self, field_tys: &[Type]) -> Type {
        unsafe {
            let mut field_tys: Vec<_> = field_tys.iter().map(|f| f.0).collect();
            Type(LLVMStructTypeInContext(
                self.0,
                field_tys.as_mut_ptr(),
                field_tys.len() as u32,
                0, /* !packed */
            ))
        }
    }

    pub fn void_type(&self) -> Type {
        unsafe { Type(LLVMVoidTypeInContext(self.0)) }
    }

    pub fn int_type(&self, len: usize) -> Type {
        unsafe { Type(LLVMIntTypeInContext(self.0, len as libc::c_uint)) }
    }

    pub fn array_type(&self, ll_elt_ty: Type, len: usize) -> Type {
        unsafe { Type(LLVMArrayType(ll_elt_ty.0, len as libc::c_uint)) }
    }

    pub fn vector_type(&self, ll_elt_ty: Type, len: usize) -> Type {
        unsafe { Type(LLVMVectorType(ll_elt_ty.0, len as libc::c_uint)) }
    }

    pub fn llvm_type_from_rust_type<T: 'static>(&self) -> Type {
        match std::any::type_name::<T>() {
            "u8" => self.int_type(8),
            "u16" => self.int_type(16),
            "u32" => self.int_type(32),
            "u64" => self.int_type(64),
            "u128" => self.int_type(128),
            _ => todo!("{}", std::any::type_name::<T>()),
        }
    }

    pub fn named_struct_type(&self, name: &str) -> Option<StructType> {
        unsafe {
            let tyref = LLVMGetTypeByName2(self.0, name.cstr());
            if tyref.is_null() {
                None
            } else {
                Some(StructType(tyref))
            }
        }
    }

    pub fn anonymous_struct_type(&self, field_tys: &[Type]) -> StructType {
        unsafe {
            let mut field_tys: Vec<_> = field_tys.iter().map(|f| f.0).collect();
            StructType(LLVMStructTypeInContext(
                self.0,
                field_tys.as_mut_ptr(),
                field_tys.len() as u32,
                0, /* !packed */
            ))
        }
    }

    pub fn create_opaque_named_struct(&self, name: &str) -> StructType {
        unsafe { StructType(LLVMStructCreateNamed(self.0, name.cstr())) }
    }

    pub fn const_string(&self, v: &str) -> ArrayValue {
        unsafe {
            ArrayValue(LLVMConstStringInContext(
                self.0,
                v.cstr(),
                v.len() as u32,
                true as i32, /* !null_terminated */
            ))
        }
    }

    pub fn const_int_array<T: PrimInt + ToPrimitive + 'static>(&self, v: &[T]) -> ArrayValue {
        let llty = self.llvm_type_from_rust_type::<T>();
        unsafe {
            let mut vals: Vec<_> = v
                .iter()
                .map(|x| Constant::int(llty, u256::U256::from((*x).to_u128().unwrap())).0)
                .collect();
            ArrayValue(LLVMConstArray(llty.0, vals.as_mut_ptr(), vals.len() as u32))
        }
    }

    pub fn const_array(&self, vals: &Vec<Constant>, llty: Type) -> ArrayValue {
        let mut llvals: Vec<_> = vals.iter().map(|v| v.get0()).collect();
        unsafe {
            ArrayValue(LLVMConstArray(
                llty.0,
                llvals.as_mut_ptr(),
                vals.len() as u32,
            ))
        }
    }

    pub fn const_struct(&self, fields: &[Constant]) -> Constant {
        unsafe {
            let mut fields: Vec<_> = fields.iter().map(|f| f.0).collect();
            Constant(LLVMConstStructInContext(
                self.0,
                fields.as_mut_ptr(),
                fields.len() as u32,
                false as i32, /* packed */
            ))
        }
    }

    pub fn const_named_struct(&self, fields: &[Constant], name: &str) -> Constant {
        unsafe {
            let tyref = LLVMGetTypeByName2(self.0, name.cstr());
            assert!(!tyref.is_null());
            let mut fields: Vec<_> = fields.iter().map(|f| f.0).collect();
            Constant(LLVMConstNamedStruct(
                tyref,
                fields.as_mut_ptr(),
                fields.len() as u32,
            ))
        }
    }

    pub fn abi_size_of_type(&self, data_layout: TargetData, ty: Type) -> usize {
        unsafe { LLVMABISizeOfType(data_layout.0, ty.0) as usize }
    }

    pub fn abi_alignment_of_type(&self, data_layout: TargetData, ty: Type) -> usize {
        unsafe { LLVMABIAlignmentOfType(data_layout.0, ty.0) as usize }
    }
}

#[derive(Copy, Clone)]
pub struct TargetData(LLVMTargetDataRef);

pub struct Module(LLVMModuleRef);

impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeModule(self.0);
        }
    }
}

impl AsMut<llvm_sys::LLVMModule> for Module {
    fn as_mut(&mut self) -> &mut llvm_sys::LLVMModule {
        unsafe { &mut *self.0 }
    }
}

impl Module {
    pub fn set_target(&self, triple: &str) {
        unsafe {
            LLVMSetTarget(self.0, triple.cstr());
        }
    }

    pub fn dump(&self) {
        unsafe {
            LLVMDumpModule(self.0);
        }
    }

    pub fn set_source_file_name(&self, name: &str) {
        unsafe { LLVMSetSourceFileName(self.0, name.as_ptr() as *const libc::c_char, name.len()) }
    }

    pub fn add_function(&self, name: &str, ty: FunctionType) -> Function {
        unsafe { Function(LLVMAddFunction(self.0, name.cstr(), ty.0)) }
    }

    pub fn add_function_with_attrs(
        &self,
        name: &str,
        ty: FunctionType,
        attrs: &[AttributeKind],
    ) -> Function {
        unsafe {
            let cx = LLVMGetModuleContext(self.0);
            let attrs = attrs
                .iter()
                .map(|a| LLVMRustCreateAttrNoValue(cx, *a))
                .collect::<Vec<_>>();

            let llfn = self.add_function(name, ty);

            AddFunctionAttributes(llfn.0, AttributePlace::Function, &attrs);

            llfn
        }
    }

    pub fn get_named_function(&self, name: &str) -> Option<Function> {
        unsafe {
            let llfn = LLVMGetNamedFunction(self.0, name.cstr());
            if !llfn.is_null() {
                Some(Function(llfn))
            } else {
                None
            }
        }
    }

    pub fn declare_known_functions(&self) {
        // Declare i32 @memcmp(ptr, ptr, i64).
        unsafe {
            let cx = LLVMGetModuleContext(self.0);
            let memcmp_arg_tys: Vec<Type> = vec![
                Type(LLVMPointerTypeInContext(cx, 0 as libc::c_uint)),
                Type(LLVMPointerTypeInContext(cx, 0 as libc::c_uint)),
                Type(LLVMInt64TypeInContext(cx)),
            ];
            let memcmp_rty = Type(LLVMInt32TypeInContext(cx));
            let memcmp_fty = FunctionType::new(memcmp_rty, &memcmp_arg_tys);
            self.add_function("memcmp", memcmp_fty);
        }
    }

    pub fn verify(&self) {
        use llvm_sys::analysis::*;
        unsafe {
            LLVMVerifyModule(
                self.0,
                LLVMVerifierFailureAction::LLVMAbortProcessAction,
                ptr::null_mut(),
            );
        }
    }

    pub fn set_data_layout(&self, machine: &TargetMachine) {
        unsafe {
            let target_data = LLVMCreateTargetDataLayout(machine.0);
            let layout_str = LLVMCopyStringRepOfTargetData(target_data);
            LLVMSetDataLayout(self.0, layout_str);
            LLVMDisposeMessage(layout_str);
            LLVMDisposeTargetData(target_data);
        }
    }

    pub fn get_module_data_layout(&self) -> TargetData {
        use log::debug;
        unsafe {
            let dl = LLVMGetModuleDataLayout(self.0);
            debug!(target: "dl", "\n{}", CStr::from_ptr(LLVMCopyStringRepOfTargetData(dl)).to_str().unwrap());
            TargetData(dl)
        }
    }

    pub fn get_global(&self, name: &str) -> Option<Global> {
        unsafe {
            let v = LLVMGetNamedGlobal(self.0, name.cstr());
            if v.is_null() {
                None
            } else {
                Some(Global(v))
            }
        }
    }

    pub fn add_global(&self, ty: Type, name: &str) -> Global {
        assert!(self.get_global(name).is_none());
        unsafe {
            let v = LLVMAddGlobal(self.0, ty.0, name.cstr());
            Global(v)
        }
    }

    pub fn add_global2(&self, ty: Type, name: &str) -> Global {
        unsafe {
            let v = LLVMAddGlobal(self.0, ty.0, name.cstr());
            Global(v)
        }
    }
}

pub struct Builder(LLVMBuilderRef);

impl Drop for Builder {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeBuilder(self.0);
        }
    }
}

impl Builder {
    pub fn get_entry_basic_block(&self, f: Function) -> BasicBlock {
        unsafe { BasicBlock(LLVMGetEntryBasicBlock(f.0)) }
    }

    pub fn position_at_beginning(&self, bb: BasicBlock) {
        unsafe {
            let inst = LLVMGetFirstInstruction(bb.0);
            LLVMPositionBuilderBefore(self.0, inst);
        }
    }

    pub fn get_insert_block(&self) -> BasicBlock {
        unsafe { BasicBlock(LLVMGetInsertBlock(self.0)) }
    }

    pub fn position_at_end(&self, bb: BasicBlock) {
        unsafe {
            LLVMPositionBuilderAtEnd(self.0, bb.0);
        }
    }

    pub fn build_alloca(&self, ty: Type, name: &str) -> Alloca {
        unsafe { Alloca(LLVMBuildAlloca(self.0, ty.0, name.cstr())) }
    }

    pub fn store_param_to_alloca(&self, param: Parameter, alloca: Alloca) {
        unsafe {
            LLVMBuildStore(self.0, param.0, alloca.0);
        }
    }

    /// Load an alloca and store in another.
    pub fn load_store(&self, ty: Type, src: Alloca, dst: Alloca) {
        unsafe {
            let tmp_reg = LLVMBuildLoad2(self.0, ty.0, src.0, "load_store_tmp".cstr());
            LLVMBuildStore(self.0, tmp_reg, dst.0);
        }
    }

    /// Reference an alloca and store it in another.
    pub fn ref_store(&self, src: Alloca, dst: Alloca) {
        unsafe {
            // allocas are pointers, so we're just storing the value of one alloca in another
            LLVMBuildStore(self.0, src.0, dst.0);
        }
    }

    /// Load a struct pointer alloca, add a field offset to it, and store the new pointer value.
    pub fn field_ref_store(&self, src: Alloca, dst: Alloca, struct_ty: StructType, offset: usize) {
        unsafe {
            let ty = src.llvm_type().0;
            let tmp_reg = LLVMBuildLoad2(self.0, ty, src.0, "tmp".cstr());
            let field_ptr = LLVMBuildStructGEP2(
                self.0,
                struct_ty.0,
                tmp_reg,
                offset as libc::c_uint,
                "fld_ref".cstr(),
            );
            LLVMBuildStore(self.0, field_ptr, dst.0);
        }
    }

    /// Add a field offset to a pointer.
    pub fn field_ref(&self, src: Alloca, struct_ty: &StructType, offset: usize) -> AnyValue {
        unsafe {
            let field_ptr = LLVMBuildStructGEP2(
                self.0,
                struct_ty.0,
                src.0,
                offset as libc::c_uint,
                "fld_ref".cstr(),
            );
            AnyValue(field_ptr)
        }
    }

    // Load the source fields, insert them into a new struct value, then store the struct value.
    pub fn insert_fields_and_store(
        &self,
        src: &[(Type, Alloca)],
        dst: (Type, Alloca),
        stype: StructType,
    ) {
        unsafe {
            let loads = src
                .iter()
                .enumerate()
                .map(|(i, (ty, val))| {
                    let name = format!("fv.{i}");
                    LLVMBuildLoad2(self.0, ty.0, val.0, name.cstr())
                })
                .collect::<Vec<_>>();

            let mut agg_val = LLVMGetUndef(stype.0);
            for (i, ld) in loads.iter().enumerate() {
                let s = format!("insert_{i}").cstr();
                agg_val = LLVMBuildInsertValue(self.0, agg_val, *ld, i as libc::c_uint, s);
            }

            assert_eq!(LLVMTypeOf(agg_val), dst.0 .0);
            LLVMBuildStore(self.0, agg_val, dst.1 .0);
        }
    }

    // Load the source struct, extract fields, then store each field in a local.
    pub fn load_and_extract_fields(
        &self,
        src: (Type, Alloca),
        dst: &[(Type, Alloca)],
        stype: StructType,
    ) {
        unsafe {
            assert_eq!(src.0 .0, stype.0);
            let srcval = LLVMBuildLoad2(self.0, stype.0, src.1 .0, "srcval".cstr());

            // The LLVM struct currently has one additional compiler-generated field. We won't
            // extract that for an unpack.
            let user_field_count = dst.len();
            assert_eq!(
                user_field_count + 1,
                LLVMCountStructElementTypes(stype.0) as usize
            );

            let mut extracts = Vec::with_capacity(user_field_count);
            for i in 0..user_field_count {
                let name = format!("ext_{i}");
                let ev = LLVMBuildExtractValue(self.0, srcval, i as libc::c_uint, name.cstr());
                extracts.push(ev);
            }

            for i in 0..user_field_count {
                assert_eq!(
                    dst[i].0 .0,
                    LLVMStructGetTypeAtIndex(stype.0, i as libc::c_uint)
                );
                LLVMBuildStore(self.0, extracts[i], dst[i].1 .0);
            }
        }
    }

    /// Load a pointer alloca, dereference, and store the value.
    pub fn load_deref_store(&self, ty: Type, src: Alloca, dst: Alloca) {
        unsafe {
            let tmp_reg1 = LLVMBuildLoad2(
                self.0,
                ty.ptr_type().0,
                src.0,
                "load_deref_store_tmp1".cstr(),
            );
            let tmp_reg2 = LLVMBuildLoad2(self.0, ty.0, tmp_reg1, "load_deref_store_tmp2".cstr());
            LLVMBuildStore(self.0, tmp_reg2, dst.0);
        }
    }

    /// Load a value from src alloca, store it to the location pointed to by dst alloca.
    pub fn load_store_ref(&self, ty: Type, src: Alloca, dst: Alloca) {
        unsafe {
            let src_reg = LLVMBuildLoad2(self.0, ty.0, src.0, "load_store_ref_src".cstr());
            let dst_ptr_reg = LLVMBuildLoad2(
                self.0,
                ty.ptr_type().0,
                dst.0,
                "load_store_ref_dst_ptr".cstr(),
            );
            LLVMBuildStore(self.0, src_reg, dst_ptr_reg);
        }
    }

    pub fn build_return_void(&self) {
        unsafe {
            LLVMBuildRetVoid(self.0);
        }
    }

    pub fn load_return(&self, ty: Type, val: Alloca) {
        unsafe {
            let tmp_reg = LLVMBuildLoad2(self.0, ty.0, val.0, "retval".cstr());
            LLVMBuildRet(self.0, tmp_reg);
        }
    }

    pub fn load_multi_return(&self, return_ty: Type, vals: &[(Type, Alloca)]) {
        unsafe {
            let loads = vals
                .iter()
                .enumerate()
                .map(|(i, (ty, val))| {
                    let name = format!("rv.{i}");
                    LLVMBuildLoad2(self.0, ty.0, val.0, name.cstr())
                })
                .collect::<Vec<_>>();

            let mut agg_val = LLVMGetUndef(return_ty.0);
            for (i, load) in loads.into_iter().enumerate() {
                let s = format!("insert_{i}").cstr();
                agg_val = LLVMBuildInsertValue(self.0, agg_val, load, i as libc::c_uint, s);
            }
            LLVMBuildRet(self.0, agg_val);
        }
    }

    pub fn store_const(&self, src: Constant, dst: Alloca) {
        unsafe {
            LLVMBuildStore(self.0, src.0, dst.0);
        }
    }

    pub fn build_br(&self, bb: BasicBlock) {
        unsafe {
            LLVMBuildBr(self.0, bb.0);
        }
    }

    pub fn build_cond_br(&self, cnd_reg: AnyValue, bb0: BasicBlock, bb1: BasicBlock) {
        unsafe {
            LLVMBuildCondBr(self.0, cnd_reg.0, bb0.0, bb1.0);
        }
    }

    pub fn load_cond_br(&self, ty: Type, val: Alloca, bb0: BasicBlock, bb1: BasicBlock) {
        unsafe {
            let cnd_reg = LLVMBuildLoad2(self.0, ty.0, val.0, "cnd".cstr());
            LLVMBuildCondBr(self.0, cnd_reg, bb0.0, bb1.0);
        }
    }

    pub fn build_extract_value(&self, agg_val: AnyValue, index: u32, name: &str) -> AnyValue {
        unsafe { AnyValue(LLVMBuildExtractValue(self.0, agg_val.0, index, name.cstr())) }
    }

    // Build call to an intrinsic (use the 'types' parameter for overloaded intrinsics).
    pub fn build_intrinsic_call(
        &self,
        module: &Module,
        iname: &str,
        types: &[Type],
        args: &[AnyValue],
        resname: &str,
    ) -> AnyValue {
        let mut tys = types.iter().map(|ty| ty.0).collect::<Vec<_>>();
        let mut args = args.iter().map(|arg| arg.0).collect::<Vec<_>>();

        unsafe {
            let iid = LLVMLookupIntrinsicID(iname.cstr(), iname.len());
            let fv = LLVMGetIntrinsicDeclaration(module.0, iid, tys.as_mut_ptr(), tys.len());
            assert_eq!(LLVMIsAFunction(fv), fv);

            let cx = LLVMGetModuleContext(module.0);
            let fnty = LLVMIntrinsicGetType(cx, iid, tys.as_mut_ptr(), tys.len());
            AnyValue(LLVMBuildCall2(
                self.0,
                fnty,
                fv,
                args.as_mut_ptr(),
                args.len() as libc::c_uint,
                resname.cstr(),
            ))
        }
    }

    pub fn load_alloca(&self, val: Alloca, ty: Type) -> AnyValue {
        unsafe {
            let name = "loaded_alloca";
            AnyValue(LLVMBuildLoad2(self.0, ty.0, val.0, name.cstr()))
        }
    }

    pub fn call(&self, fnval: Function, args: &[AnyValue]) -> AnyValue {
        let fnty = fnval.llvm_type();

        unsafe {
            let mut args = args.iter().map(|val| val.0).collect::<Vec<_>>();
            AnyValue(LLVMBuildCall2(
                self.0,
                fnty.0,
                fnval.0,
                args.as_mut_ptr(),
                args.len() as libc::c_uint,
                "".cstr(),
            ))
        }
    }

    pub fn call_store(&self, fnval: Function, args: &[AnyValue], dst: &[(Type, Alloca)]) {
        let fnty = fnval.llvm_type();

        unsafe {
            let mut args = args.iter().map(|a| a.0).collect::<Vec<_>>();
            let ret = LLVMBuildCall2(
                self.0,
                fnty.0,
                fnval.0,
                args.as_mut_ptr(),
                args.len() as libc::c_uint,
                (if dst.is_empty() { "" } else { "retval" }).cstr(),
            );

            if dst.is_empty() {
                // No return values.
            } else if dst.len() == 1 {
                // Single return value.
                LLVMBuildStore(self.0, ret, dst[0].1 .0);
            } else {
                // Multiple return values-- unwrap the struct.
                let extracts = dst
                    .iter()
                    .enumerate()
                    .map(|(i, (_ty, dval))| {
                        let name = format!("extract_{i}");
                        let ev = LLVMBuildExtractValue(self.0, ret, i as libc::c_uint, name.cstr());
                        (ev, dval)
                    })
                    .collect::<Vec<_>>();
                for (ev, dval) in extracts {
                    LLVMBuildStore(self.0, ev, dval.0);
                }
            }
        }
    }

    pub fn load_call_store(
        &self,
        fnval: Function,
        args: &[(Type, Alloca)],
        dst: &[(Type, Alloca)],
    ) {
        unsafe {
            let args = args
                .iter()
                .enumerate()
                .map(|(i, (ty, val))| {
                    let name = format!("call_arg_{i}");
                    AnyValue(LLVMBuildLoad2(self.0, ty.0, val.0, name.cstr()))
                })
                .collect::<Vec<_>>();
            self.call_store(fnval, &args, dst)
        }
    }

    pub fn build_call_imm(&self, fnval: Function, args: &[Constant]) {
        let fnty = fnval.llvm_type();
        unsafe {
            let mut args = args
                .iter()
                .enumerate()
                .map(|(_i, val)| val.0)
                .collect::<Vec<_>>();
            LLVMBuildCall2(
                self.0,
                fnty.0,
                fnval.0,
                args.as_mut_ptr(),
                args.len() as libc::c_uint,
                "".cstr(),
            );
        }
    }

    pub fn build_unreachable(&self) {
        unsafe {
            LLVMBuildUnreachable(self.0);
        }
    }

    pub fn build_load(&self, ty: Type, src0_reg: Alloca, name: &str) -> AnyValue {
        unsafe { AnyValue(LLVMBuildLoad2(self.0, ty.0, src0_reg.0, name.cstr())) }
    }

    pub fn build_load_from_valref(&self, ty: Type, src0_reg: AnyValue, name: &str) -> AnyValue {
        unsafe { AnyValue(LLVMBuildLoad2(self.0, ty.0, src0_reg.0, name.cstr())) }
    }

    pub fn build_load_global_const(&self, gval: Global) -> Constant {
        unsafe {
            let ty = LLVMGlobalGetValueType(gval.0);
            Constant(LLVMBuildLoad2(self.0, ty, gval.0, "".cstr()))
        }
    }

    pub fn build_store(&self, dst_reg: AnyValue, dst: Alloca) {
        unsafe {
            LLVMBuildStore(self.0, dst_reg.0, dst.0);
        }
    }

    #[allow(dead_code)]
    pub fn load_add_store(&self, ty: Type, src0: Alloca, src1: Alloca, dst: Alloca) {
        unsafe {
            let src0_reg = LLVMBuildLoad2(self.0, ty.0, src0.0, "add_src_0".cstr());
            let src1_reg = LLVMBuildLoad2(self.0, ty.0, src1.0, "add_src_1".cstr());
            let dst_reg = LLVMBuildAdd(self.0, src0_reg, src1_reg, "add_dst".cstr());
            LLVMBuildStore(self.0, dst_reg, dst.0);
        }
    }

    // TODO: If \p name isn't provided get a tempname.
    pub fn build_binop(
        &self,
        op: LLVMOpcode,
        lhs: AnyValue,
        rhs: AnyValue,
        name: &str,
    ) -> AnyValue {
        unsafe { AnyValue(LLVMBuildBinOp(self.0, op, lhs.0, rhs.0, name.cstr())) }
    }
    pub fn build_compare(
        &self,
        pred: LLVMIntPredicate,
        lhs: AnyValue,
        rhs: AnyValue,
        name: &str,
    ) -> AnyValue {
        unsafe { AnyValue(LLVMBuildICmp(self.0, pred, lhs.0, rhs.0, name.cstr())) }
    }
    #[allow(dead_code)]
    pub fn build_unary_bitcast(&self, val: AnyValue, dest_ty: Type, name: &str) -> AnyValue {
        unsafe { AnyValue(LLVMBuildBitCast(self.0, val.0, dest_ty.0, name.cstr())) }
    }
    pub fn build_zext(&self, val: AnyValue, dest_ty: Type, name: &str) -> AnyValue {
        unsafe { AnyValue(LLVMBuildZExt(self.0, val.0, dest_ty.0, name.cstr())) }
    }
    pub fn build_trunc(&self, val: AnyValue, dest_ty: Type, name: &str) -> AnyValue {
        unsafe { AnyValue(LLVMBuildTrunc(self.0, val.0, dest_ty.0, name.cstr())) }
    }

    pub fn wrap_as_any_value(&self, val: LLVMValueRef) -> AnyValue {
        AnyValue(val)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Type(pub LLVMTypeRef);

impl Type {
    pub fn ptr_type(&self) -> Type {
        unsafe { Type(LLVMPointerType(self.0, 0)) }
    }

    pub fn as_struct_type(&self) -> StructType {
        StructType(self.0)
    }

    pub fn get_int_type_width(&self) -> u32 {
        unsafe { LLVMGetIntTypeWidth(self.0) }
    }

    pub fn is_integer_ty(&self) -> bool {
        unsafe { LLVMGetTypeKind(self.0) == LLVMIntegerTypeKind }
    }

    pub fn get_context(&self) -> Context {
        unsafe { Context(LLVMGetTypeContext(self.0)) }
    }

    pub fn get_array_length(&self) -> usize {
        unsafe { LLVMGetArrayLength(self.0) as usize }
    }

    pub fn get_element_type(&self) -> Type {
        unsafe { Type(LLVMGetElementType(self.0)) }
    }

    pub fn dump(&self) {
        unsafe {
            LLVMDumpType(self.0);
            eprintln!();
        }
    }

    pub fn dump_properties_to_str(&self, data_layout: TargetData) -> String {
        unsafe {
            let ty = self.0;
            let s = &format!(
                "StoreSizeOfType: {}\nABISizeOfType: {}\nABIAlignmnetOfType: {}\nSizeOfTypeInBits: {}\n",
                LLVMStoreSizeOfType(data_layout.0, ty) as u32,
                LLVMABISizeOfType(data_layout.0, ty) as u32,
                LLVMABIAlignmentOfType(data_layout.0, ty),
                LLVMSizeOfTypeInBits(data_layout.0, ty) as u32,
            );
            s.to_string()
        }
    }

    pub fn print_to_str(&self) -> &str {
        unsafe {
            CStr::from_ptr(LLVMPrintTypeToString(self.0))
                .to_str()
                .unwrap()
        }
    }
}

pub struct StructType(LLVMTypeRef);

impl StructType {
    pub fn as_any_type(&self) -> Type {
        Type(self.0)
    }

    pub fn ptr_type(&self) -> Type {
        unsafe { Type(LLVMPointerType(self.0, 0)) }
    }

    pub fn get_context(&self) -> Context {
        unsafe { Context(LLVMGetTypeContext(self.0)) }
    }

    pub fn set_struct_body(&self, field_tys: &[Type]) {
        unsafe {
            let mut field_tys: Vec<_> = field_tys.iter().map(|f| f.0).collect();
            LLVMStructSetBody(
                self.0,
                field_tys.as_mut_ptr(),
                field_tys.len() as u32,
                0, /* !packed */
            );
        }
    }

    pub fn count_struct_element_types(&self) -> usize {
        unsafe { LLVMCountStructElementTypes(self.0) as usize }
    }

    pub fn struct_get_type_at_index(&self, idx: usize) -> Type {
        unsafe { Type(LLVMStructGetTypeAtIndex(self.0, idx as libc::c_uint)) }
    }

    pub fn offset_of_element(&self, data_layout: TargetData, idx: usize) -> usize {
        unsafe { LLVMOffsetOfElement(data_layout.0, self.0, idx as libc::c_uint) as usize }
    }

    pub fn dump(&self) {
        unsafe {
            LLVMDumpType(self.0);
            eprintln!();
        }
    }
}

#[derive(Copy, Clone)]
pub struct FunctionType(LLVMTypeRef);

impl FunctionType {
    pub fn new(return_type: Type, parameter_types: &[Type]) -> FunctionType {
        let mut parameter_types: Vec<_> = parameter_types.iter().map(|t| t.0).collect();
        unsafe {
            FunctionType(LLVMFunctionType(
                return_type.0,
                parameter_types.as_mut_ptr(),
                parameter_types.len() as libc::c_uint,
                false as LLVMBool,
            ))
        }
    }
}

#[derive(Copy, Clone)]
pub struct Function(LLVMValueRef);

impl Function {
    pub fn as_gv(&self) -> Global {
        Global(self.0)
    }

    pub fn get_next_basic_block(&self, basic_block: BasicBlock) -> Option<BasicBlock> {
        let next_bb = unsafe { BasicBlock(LLVMGetNextBasicBlock(basic_block.0)) };
        if next_bb.0.is_null() {
            return None;
        }
        Some(next_bb)
    }

    pub fn append_basic_block(&self, name: &str) -> BasicBlock {
        unsafe { BasicBlock(LLVMAppendBasicBlock(self.0, name.cstr())) }
    }

    pub fn prepend_basic_block(&self, basic_block: BasicBlock, name: &str) -> BasicBlock {
        unsafe { BasicBlock(LLVMInsertBasicBlock(basic_block.0, name.cstr())) }
    }

    pub fn insert_basic_block_after(&self, basic_block: BasicBlock, name: &str) -> BasicBlock {
        match self.get_next_basic_block(basic_block) {
            Some(bb) => self.prepend_basic_block(bb, name),
            None => self.append_basic_block(name),
        }
    }

    pub fn get_param(&self, i: usize) -> Parameter {
        unsafe { Parameter(LLVMGetParam(self.0, i as u32)) }
    }

    pub fn llvm_type(&self) -> FunctionType {
        unsafe { FunctionType(LLVMGlobalGetValueType(self.0)) }
    }

    pub fn llvm_return_type(&self) -> Type {
        unsafe { Type(LLVMGetReturnType(LLVMGlobalGetValueType(self.0))) }
    }

    pub fn verify(&self) {
        use llvm_sys::analysis::*;
        unsafe {
            LLVMVerifyFunction(self.0, LLVMVerifierFailureAction::LLVMAbortProcessAction);
        }
    }
}

#[derive(Copy, Clone)]
pub struct BasicBlock(LLVMBasicBlockRef);

impl BasicBlock {
    pub fn get_basic_block_parent(&self) -> Function {
        unsafe { Function(LLVMGetBasicBlockParent(self.0)) }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Alloca(LLVMValueRef);

impl Alloca {
    pub fn as_any_value(&self) -> AnyValue {
        AnyValue(self.0)
    }

    pub fn as_constant(&self) -> Constant {
        Constant(self.0)
    }

    pub fn llvm_type(&self) -> Type {
        unsafe { Type(LLVMTypeOf(self.0)) }
    }
    pub fn get0(&self) -> LLVMValueRef {
        self.0
    }
}

#[derive(Copy, Clone)]
pub struct AnyValue(LLVMValueRef);

impl AnyValue {
    pub fn get0(&self) -> LLVMValueRef {
        self.0
    }

    pub fn llvm_type(&self) -> Type {
        unsafe { Type(LLVMTypeOf(self.0)) }
    }

    pub fn as_constant(&self) -> Constant {
        Constant(self.0)
    }

    pub fn dump(&self) {
        unsafe {
            LLVMDumpValue(self.0);
        }
    }
}

#[derive(Copy, Clone)]
pub struct Global(LLVMValueRef);

impl Global {
    pub fn ptr(&self) -> Constant {
        Constant(self.0)
    }

    pub fn as_any_value(&self) -> AnyValue {
        AnyValue(self.0)
    }

    pub fn set_constant(&self) {
        unsafe {
            LLVMSetGlobalConstant(self.0, true as i32);
        }
    }

    pub fn set_linkage(&self, linkage: LLVMLinkage) {
        unsafe {
            LLVMSetLinkage(self.0, linkage);
        }
    }

    pub fn set_unnamed_addr(&self) {
        unsafe {
            LLVMSetUnnamedAddress(self.0, LLVMUnnamedAddr::LLVMGlobalUnnamedAddr);
        }
    }

    pub fn set_initializer(&self, v: Constant) {
        unsafe {
            LLVMSetInitializer(self.0, v.0);
        }
    }

    pub fn set_internal_linkage(&self) {
        unsafe {
            LLVMSetLinkage(self.0, LLVMInternalLinkage);
        }
    }

    pub fn dump(&self) {
        unsafe {
            LLVMDumpValue(self.0);
            eprintln!();
        }
    }
}

pub struct Parameter(LLVMValueRef);

impl Parameter {}

pub struct Constant(LLVMValueRef);

impl Constant {
    pub fn as_any_value(&self) -> AnyValue {
        AnyValue(self.0)
    }

    pub fn int(ty: Type, v: u256::U256) -> Constant {
        unsafe {
            let val_as_str = format!("{v}");
            Constant(LLVMConstIntOfString(ty.0, val_as_str.cstr(), 10))
        }
    }

    pub fn get_const_null(ty: Type) -> Constant {
        unsafe { Constant(LLVMConstNull(ty.0)) }
    }

    pub fn get0(&self) -> LLVMValueRef {
        self.0
    }

    pub fn llvm_type(&self) -> Type {
        unsafe { Type(LLVMTypeOf(self.0)) }
    }

    pub fn dump(&self) {
        unsafe {
            LLVMDumpValue(self.0);
            eprintln!();
        }
    }
}

pub struct ArrayValue(LLVMValueRef);

impl ArrayValue {
    pub fn as_const(&self) -> Constant {
        Constant(self.0)
    }

    pub fn llvm_type(&self) -> Type {
        unsafe { Type(LLVMTypeOf(self.0)) }
    }
}

pub struct Target(LLVMTargetRef);

impl Target {
    pub fn from_triple(triple: &str) -> anyhow::Result<Target> {
        unsafe {
            let target: &mut LLVMTargetRef = &mut ptr::null_mut();
            let error: &mut *mut libc::c_char = &mut ptr::null_mut();
            let result = LLVMGetTargetFromTriple(triple.cstr(), target, error);

            if result == 0 {
                assert!((*error).is_null());
                Ok(Target(*target))
            } else {
                assert!(!(*error).is_null());
                let rust_error = CStr::from_ptr(*error).to_str()?.to_string();
                LLVMDisposeMessage(*error);
                anyhow::bail!("{rust_error}");
            }
        }
    }

    pub fn create_target_machine(&self, triple: &str, cpu: &str, features: &str) -> TargetMachine {
        unsafe {
            // fixme some of these should be params
            let level = LLVMCodeGenOptLevel::LLVMCodeGenLevelNone;
            let reloc = LLVMRelocMode::LLVMRelocPIC;
            let code_model = LLVMCodeModel::LLVMCodeModelDefault;

            let machine = LLVMCreateTargetMachine(
                self.0,
                triple.cstr(),
                cpu.cstr(),
                features.cstr(),
                level,
                reloc,
                code_model,
            );

            TargetMachine(machine)
        }
    }
}

pub struct TargetMachine(LLVMTargetMachineRef);

impl Drop for TargetMachine {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeTargetMachine(self.0);
        }
    }
}

impl TargetMachine {
    pub fn emit_to_obj_file(&self, module: &Module, filename: &str) -> anyhow::Result<()> {
        unsafe {
            // nb: llvm-sys seemingly-incorrectly wants
            // a mutable c-string for the filename.
            let filename = CString::new(filename.to_string()).expect("interior nul byte");
            let mut filename = filename.into_bytes_with_nul();
            let filename: *mut u8 = filename.as_mut_ptr();
            let filename = filename as *mut libc::c_char;

            let error: &mut *mut libc::c_char = &mut ptr::null_mut();
            let result = LLVMTargetMachineEmitToFile(
                self.0,
                module.0,
                filename,
                LLVMCodeGenFileType::LLVMObjectFile,
                error,
            );

            if result == 0 {
                assert!((*error).is_null());
                Ok(())
            } else {
                assert!(!(*error).is_null());
                let rust_error = CStr::from_ptr(*error).to_str()?.to_string();
                LLVMDisposeMessage(*error);
                anyhow::bail!("{rust_error}");
            }
        }
    }
}
