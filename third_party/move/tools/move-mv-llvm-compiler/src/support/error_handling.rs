// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module contains some supplemental functions for dealing with errors.

#![allow(unused)]

use libc::c_void;
use llvm_sys::{
    core::{LLVMGetDiagInfoDescription, LLVMGetDiagInfoSeverity},
    error_handling::{LLVMInstallFatalErrorHandler, LLVMResetFatalErrorHandler},
    prelude::LLVMDiagnosticInfoRef,
    LLVMDiagnosticSeverity,
};

#[cfg(feature = "internal-getters")]
use crate::LLVMReference;

// REVIEW: Maybe it's possible to have a safe wrapper? If we can
// wrap the provided function input ptr into a &CStr somehow
// TODOC: Can be used like this:
// extern "C" fn print_before_exit(msg: *const i8) {
//    let c_str = unsafe { ::std::ffi::CStr::from_ptr(msg) };
//
//    eprintln!("LLVM fatally errored: {:?}", c_str);
// }
// unsafe {
//     install_fatal_error_handler(print_before_exit);
// }
// and will be called before LLVM calls C exit()
/// Installs an error handler to be called before LLVM exits.
///
/// # Safety
///
/// Unclear.
pub unsafe fn install_fatal_error_handler(handler: extern "C" fn(*const ::libc::c_char)) {
    LLVMInstallFatalErrorHandler(Some(handler))
}

/// Resets LLVM's fatal error handler back to the default
pub fn reset_fatal_error_handler() {
    unsafe { LLVMResetFatalErrorHandler() }
}

pub(crate) struct DiagnosticInfo {
    diagnostic_info: LLVMDiagnosticInfoRef,
}

impl DiagnosticInfo {
    pub(crate) fn new(diagnostic_info: LLVMDiagnosticInfoRef) -> Self {
        DiagnosticInfo { diagnostic_info }
    }

    pub(crate) fn get_description(&self) -> *mut ::libc::c_char {
        unsafe { LLVMGetDiagInfoDescription(self.diagnostic_info) }
    }

    pub(crate) fn severity_is_error(&self) -> bool {
        unsafe {
            matches!(
                LLVMGetDiagInfoSeverity(self.diagnostic_info),
                LLVMDiagnosticSeverity::LLVMDSError
            )
        }
    }
}

// Assmuptions this handler makes:
// * A valid *mut *mut i8 is provided as the void_ptr (via context.set_diagnostic_handler)
//
// https://github.com/llvm-mirror/llvm/blob/master/tools/llvm-c-test/diagnostic.c was super useful
// for figuring out how to get this to work
pub(crate) extern "C" fn get_error_str_diagnostic_handler(
    diagnostic_info: LLVMDiagnosticInfoRef,
    void_ptr: *mut c_void,
) {
    let diagnostic_info = DiagnosticInfo::new(diagnostic_info);

    if diagnostic_info.severity_is_error() {
        let c_ptr_ptr = void_ptr as *mut *mut c_void as *mut *mut ::libc::c_char;

        unsafe {
            *c_ptr_ptr = diagnostic_info.get_description();
        }
    }
}

#[cfg(feature = "internal-getters")]
impl LLVMReference<LLVMDiagnosticInfoRef> for DiagnosticInfo {
    unsafe fn get_ref(&self) -> LLVMDiagnosticInfoRef {
        self.diagnostic_info
    }
}
