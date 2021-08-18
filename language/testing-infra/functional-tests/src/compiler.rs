// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_binary_format::file_format::{CompiledModule, CompiledScript};

pub trait Compiler {
    /// Compile a transaction script or module.
    fn compile<Logger: FnMut(String)>(
        &mut self,
        log: Logger,
        input: &str,
    ) -> Result<ScriptOrModule>;

    fn use_compiled_genesis(&self) -> bool;
}

pub enum ScriptOrModule {
    Script(Option<Vec<u8>>, CompiledScript),
    Module(CompiledModule),
}
