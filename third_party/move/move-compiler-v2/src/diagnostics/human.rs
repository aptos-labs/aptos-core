// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::diagnostics::Emitter;
use codespan::{FileId, Files};
use codespan_reporting::{
    diagnostic::Diagnostic,
    term::{emit, termcolor::WriteColor, Config},
};

/// It's used in the native velor-cli output to show error messages.
/// Wraps the `codespan_reporting::term::emit()` method.
pub struct HumanEmitter<'w, W: WriteColor> {
    writer: &'w mut W,
}

impl<'w, W> HumanEmitter<'w, W>
where
    W: WriteColor,
{
    pub fn new(writer: &'w mut W) -> Self {
        HumanEmitter { writer }
    }
}

impl<W> Emitter for HumanEmitter<'_, W>
where
    W: WriteColor,
{
    fn emit(&mut self, source_files: &Files<String>, diag: &Diagnostic<FileId>) {
        emit(&mut self.writer, &Config::default(), source_files, diag).expect("emit must not fail")
    }
}
