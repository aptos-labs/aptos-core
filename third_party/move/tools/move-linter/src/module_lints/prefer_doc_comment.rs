// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint (strict): regular `//` comments immediately before a documentable item should be `///`.

use codespan::{ByteIndex, Span};
use move_compiler_v2::external_checks::ModuleChecker;
use move_model::model::{FunctionEnv, GlobalEnv, Loc, NamedConstantEnv, StructEnv};

pub struct PreferDocComment;

impl ModuleChecker for PreferDocComment {
    fn get_name(&self) -> String {
        "prefer_doc_comment".to_string()
    }

    fn visit_function(&self, env: &GlobalEnv, func: &FunctionEnv) {
        check_for_regular_comment(self, env, &func.get_id_loc());
    }

    fn visit_named_constant(&self, env: &GlobalEnv, constant: &NamedConstantEnv) {
        check_for_regular_comment(self, env, &constant.get_loc());
    }

    fn visit_struct(&self, env: &GlobalEnv, struct_env: &StructEnv) {
        check_for_regular_comment(self, env, &struct_env.get_loc());
    }
}

/// Check if there are regular `//` comments (not `///`) immediately preceding the item at `loc`.
fn check_for_regular_comment(checker: &PreferDocComment, env: &GlobalEnv, loc: &Loc) {
    let file_id = loc.file_id();
    let source = env.get_file_source(file_id);
    let item_start = loc.span().start().to_usize();

    // Find the start of the line containing the item.
    let line_start = source[..item_start]
        .rfind('\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);

    // Now scan backwards from the line before the item to find comment lines.
    // We want to find regular // comments that are immediately preceding (no blank line gap).
    let mut scan_pos = line_start;
    let mut found_regular_comment = false;
    let mut comment_start = scan_pos;
    let mut comment_end = scan_pos;

    loop {
        if scan_pos == 0 {
            break;
        }
        // Go to the previous line.
        let prev_line_end = scan_pos; // this is the start of the current line = end of prev content
                                      // Find start of the previous line.
        let prev_line_start = if scan_pos >= 2 {
            // scan_pos - 1 should be '\n', so look before that
            source[..scan_pos - 1]
                .rfind('\n')
                .map(|pos| pos + 1)
                .unwrap_or(0)
        } else {
            0
        };

        let prev_line = &source[prev_line_start..prev_line_end];
        let trimmed = prev_line.trim();

        if trimmed.is_empty() {
            // Blank line — stop scanning.
            break;
        }

        if trimmed.starts_with("///") {
            // Doc comment — this is fine, skip past it and keep scanning.
            scan_pos = prev_line_start;
            continue;
        }

        if trimmed.starts_with("//") && !trimmed.starts_with("///") {
            // Regular comment that's not a doc comment.
            found_regular_comment = true;
            comment_start = prev_line_start;
            if comment_end == line_start {
                comment_end = prev_line_end;
            }
            scan_pos = prev_line_start;
            continue;
        }

        // Attributes (lines starting with #[) — skip past them and keep scanning.
        if trimmed.starts_with("#[") {
            scan_pos = prev_line_start;
            continue;
        }

        // Any other content — stop scanning.
        break;
    }

    if found_regular_comment {
        let span = Span::new(
            ByteIndex(comment_start as u32),
            ByteIndex(comment_end as u32),
        );
        let comment_loc = Loc::new(file_id, span);
        checker.report(
            env,
            &comment_loc,
            "Use doc comment `///` instead of regular comment `//` for documentable items.",
        );
    }
}
