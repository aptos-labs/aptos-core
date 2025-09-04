// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use annotate_snippets::{Level, Renderer, Snippet};
use std::fmt::Write;

/// Renders a formatted error message for a toml deserialization failure, optionally with
/// source span annotations.
///
/// If the error contains span information, this function highlights the relevant portion
/// of the original text using a diagnostic-style output. Otherwise, it falls back to a
/// plain error message.
///
/// # Example Output
///
/// ```text
/// error: failed to parse manifest
///   |
/// 1 | [package]
/// 2 | name = "some_package_name"
/// 3 | version = "0.1.2"
/// 4 | upgrade_policy = "invalid-policy"
///   |                  ^^^^^^^^^^^^^^^^ unknown variant `invalid-policy`, expected `compatible` or `immutable`
///   |
/// ```
pub fn render_error(
    output: &mut impl Write,
    manifest_text: &str,
    err: &toml::de::Error,
) -> anyhow::Result<()> {
    match err.span() {
        Some(span) => {
            let snippet = Snippet::source(manifest_text)
                .annotation(Level::Error.span(span).label(err.message()));

            writeln!(
                output,
                "{}",
                Renderer::plain().render(
                    Level::Error
                        .title("failed to parse manifest")
                        .snippet(snippet)
                )
            )?;
        },
        None => {
            writeln!(output, "{}", err.message())?;
            writeln!(output, "(no span info)")?;
        },
    }

    Ok(())
}
