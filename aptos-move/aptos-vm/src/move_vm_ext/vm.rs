// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{SessionExt, SessionId},
    natives::aptos_natives,
};
use move_binary_format::errors::VMResult;
use move_core_types::resolver::MoveResolver;
use move_vm_runtime::{move_vm::MoveVM, native_functions::NativeContextExtensions};
use std::ops::Deref;

pub struct MoveVmExt {
    inner: MoveVM,
}

impl MoveVmExt {
    pub fn new() -> VMResult<Self> {
        Ok(Self {
            inner: MoveVM::new(aptos_natives())?,
        })
    }

    pub fn new_session<'r, S: MoveResolver>(
        &self,
        remote: &'r S,
        _session_id: SessionId,
    ) -> SessionExt<'r, '_, S> {
        // TODO: install table extension
        let extensions = NativeContextExtensions::default();

        SessionExt::new(self.inner.new_session_with_extensions(remote, extensions))
    }
}

impl Deref for MoveVmExt {
    type Target = MoveVM;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
