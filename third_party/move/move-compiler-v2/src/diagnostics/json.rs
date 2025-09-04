// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::diagnostics::Emitter;
use codespan::{FileId, Files};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use std::io::Write;

/// Shows compiler errors as a structured JSON output.
/// Exists to support various tools external to the velor-cli, i.e. IDEs.
pub struct JsonEmitter<'w, W: Write> {
    writer: &'w mut W,
}

impl<'w, W: Write> JsonEmitter<'w, W> {
    pub fn new(writer: &'w mut W) -> Self {
        JsonEmitter { writer }
    }
}

impl<W: Write> Emitter for JsonEmitter<'_, W> {
    fn emit(&mut self, source_files: &Files<String>, diag: &Diagnostic<FileId>) {
        let fpath_labels = diag
            .labels
            .iter()
            .map(|label| {
                let fpath = codespan_reporting::files::Files::name(source_files, label.file_id)
                    .expect("always Ok() in the impl")
                    .to_string();
                Label::new(label.style, fpath, label.range.clone())
            })
            .collect();
        let mut json_diag = Diagnostic::new(diag.severity)
            .with_message(diag.message.clone())
            .with_labels(fpath_labels)
            .with_notes(diag.notes.clone());
        if let Some(code) = &diag.code {
            json_diag = json_diag.with_code(code)
        }
        serde_json::to_writer(&mut self.writer, &json_diag).expect("it should be serializable");
        writeln!(&mut self.writer)
            .expect("dest is stderr / in-memory buffer, it should always be available");
    }
}
