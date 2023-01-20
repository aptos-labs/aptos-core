// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use x_lint::prelude::*;

static DIEM_CORE_CONTRIBUTORS: &str = "Copyright (c) The Diem Core Contributors";
static MOVE_CONTRIBUTORS: &str = "Copyright (c) The Move Contributors";
static LICENSE_IDENTIFIER: &str = "SPDX-License-Identifier: Apache-2.0";

fn has_license<'a>(mut lines: impl Iterator<Item = &'a str>) -> bool {
    let first = match lines.next() {
        Some(line) => line,
        None => return false,
    };
    let maybe_move_line = if first == DIEM_CORE_CONTRIBUTORS {
        match lines.next() {
            Some(line) => line,
            None => return false,
        }
    } else {
        first
    };
    let maybe_license_identifier = match lines.next() {
        Some(line) => line,
        None => return false,
    };
    maybe_move_line == MOVE_CONTRIBUTORS && maybe_license_identifier == LICENSE_IDENTIFIER
}

#[derive(Copy, Clone, Debug)]
pub(super) struct LicenseHeader;

impl Linter for LicenseHeader {
    fn name(&self) -> &'static str {
        "license-header"
    }
}

impl ContentLinter for LicenseHeader {
    fn pre_run<'l>(&self, file_ctx: &FilePathContext<'l>) -> Result<RunStatus<'l>> {
        // TODO: Add a way to pass around state between pre_run and run, so that this computation
        // only needs to be done once.
        match FileType::new(file_ctx) {
            Some(_) => Ok(RunStatus::Executed),
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
                    .map(|s| s.trim_start_matches("// "));
                !has_license(maybe_license)
            }
            FileType::Shell => {
                let maybe_license = content
                    .lines()
                    .skip_while(|line| line.starts_with("#!"))
                    .skip_while(|line| line.is_empty())
                    .map(|s| s.trim_start_matches("# "));
                !has_license(maybe_license)
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
