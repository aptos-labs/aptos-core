// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use globset::{Glob, GlobSet, GlobSetBuilder};
use x_lint::prelude::*;

static LICENSE_HEADER: &str = "Copyright (c) Aptos\n\
                               SPDX-License-Identifier: Apache-2.0\n\
                               ";

#[derive(Copy, Clone, Debug)]
pub(super) struct LicenseHeader<'cfg> {
    exceptions: &'cfg GlobSet,
}

impl<'cfg> Linter for LicenseHeader<'cfg> {
    fn name(&self) -> &'static str {
        "license-header"
    }
}

impl<'cfg> LicenseHeader<'cfg> {
    pub fn new(exceptions: &'cfg GlobSet) -> Self {
        Self { exceptions }
    }
}

impl<'cfg> ContentLinter for LicenseHeader<'cfg> {
    fn pre_run<'l>(&self, file_ctx: &FilePathContext<'l>) -> Result<RunStatus<'l>> {
        // TODO: Add a way to pass around state between pre_run and run, so that this computation
        // only needs to be done once.
        match FileType::new(file_ctx) {
            Some(_) => Ok(skip_license_checks(self.exceptions, file_ctx)),
            None => Ok(RunStatus::Skipped(SkipReason::UnsupportedExtension(
                file_ctx.extension(),
            ))),
        }
    }

    fn run<'l>(
        &self,
        ctx: &ContentContext<'l>,
        out: &mut LintFormatter<'l, '_>,
    ) -> Result<RunStatus<'l>> {
        let content = match ctx.content() {
            Some(content) => content,
            None => {
                // This is not a UTF-8 file -- don't analyze it.
                return Ok(RunStatus::Skipped(SkipReason::NonUtf8Content));
            }
        };

        let file_type = FileType::new(ctx.file_ctx()).expect("None filtered out in pre_run");
        // Determine if the file is missing the license header
        let missing_header = match file_type {
            FileType::Rust | FileType::Proto => {
                let maybe_license = content
                    .lines()
                    .skip_while(|line| line.is_empty())
                    .take(2)
                    .map(|s| s.trim_start_matches("// "));
                !LICENSE_HEADER.lines().eq(maybe_license)
            }
            FileType::Shell => {
                let maybe_license = content
                    .lines()
                    .skip_while(|line| line.starts_with("#!"))
                    .skip_while(|line| line.is_empty())
                    .take(2)
                    .map(|s| s.trim_start_matches("# "));
                !LICENSE_HEADER.lines().eq(maybe_license)
            }
        };

        if missing_header {
            out.write(LintLevel::Error, "missing license header");
        }

        Ok(RunStatus::Executed)
    }
}

enum FileType {
    Rust,
    Shell,
    Proto,
}

impl FileType {
    fn new(ctx: &FilePathContext<'_>) -> Option<Self> {
        match ctx.extension() {
            Some("rs") => Some(FileType::Rust),
            Some("sh") => Some(FileType::Shell),
            Some("proto") => Some(FileType::Proto),
            _ => None,
        }
    }
}

pub(super) fn build_exceptions(patterns: &[String]) -> crate::Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).with_context(|| {
            format!(
                "error while processing license exception glob '{}'",
                pattern
            )
        })?;
        builder.add(glob);
    }
    builder
        .build()
        .with_context(|| "error while building globset for license patterns")
}

fn skip_license_checks<'l>(exceptions: &GlobSet, file: &FilePathContext<'l>) -> RunStatus<'l> {
    if exceptions.is_match(file.file_path()) {
        return RunStatus::Skipped(SkipReason::UnsupportedFile(file.file_path()));
    }

    RunStatus::Executed
}
