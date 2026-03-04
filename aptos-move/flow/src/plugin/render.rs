// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{Context, Result};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tera::Tera;
use walkdir::WalkDir;

/// Content directories to walk for template files.
///
/// Each entry maps a source directory (relative to the content root) to the
/// output prefix used when emitting rendered files. This allows the on-disk
/// layout (`cont/agents/`) to differ from the output layout (`agents/`).
///
/// Entries whose output prefix is `"templates"` are registered for
/// `{% include %}` but not emitted as standalone output files.
const CONTENT_DIRS: &[(&str, &str)] = &[
    ("cont/templates", "templates"),
    ("cont/agents", "agents"),
    ("cont/skills", "skills"),
    ("cont/hooks", "hooks"),
];

/// Discover and render all content files under the given root directory.
///
/// The caller-provided `context` supplies all template variables (tool target,
/// display name, version, etc.).
///
/// Returns a list of `(relative_path, rendered_content)` pairs. The relative
/// path preserves the original directory structure (e.g. `commands/example.md`).
/// Templates under `templates/` are available for `{% include %}` but are not
/// included in the output.
pub fn render_all(
    content_root: &Path,
    context: &tera::Context,
    tool_names: &[String],
) -> Result<Vec<(PathBuf, String)>> {
    let mut tera = Tera::default();
    tera.register_function("tool", make_tool_function(tool_names.to_vec()));
    let once_seen = make_once_function(&mut tera);

    // First pass: register all templates in a shared Tera instance.
    let mut output_names = Vec::new();
    for &(src_dir, out_prefix) in CONTENT_DIRS {
        let dir_path = content_root.join(src_dir);
        if !dir_path.is_dir() {
            continue;
        }

        for entry in WalkDir::new(&dir_path).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let abs_path = entry.path();
            let rel_within = abs_path
                .strip_prefix(&dir_path)
                .context("failed to compute relative path")?;
            let out_path = Path::new(out_prefix).join(rel_within);
            let template_name = out_path.to_string_lossy().to_string();

            let raw_content = std::fs::read_to_string(abs_path)
                .with_context(|| format!("failed to read {}", abs_path.display()))?;

            tera.add_raw_template(&template_name, &raw_content)
                .with_context(|| format!("failed to parse template {}", abs_path.display()))?;

            // Only emit templates that are not partials.
            if out_prefix != "templates" {
                output_names.push((out_path, template_name));
            }
        }
    }

    // Second pass: render output-producing templates.
    let mut results = Vec::new();
    for (out_path, template_name) in output_names {
        // Reset include onces so each output file deduplicates independently.
        once_seen.lock().unwrap().clear();
        let rendered = tera
            .render(&template_name, context)
            .with_context(|| format!("failed to render template {}", out_path.display()))?;
        results.push((out_path, rendered));
    }

    Ok(results)
}

/// Render a single template string through Tera.
///
/// If the content contains no Tera constructs, Tera returns it unchanged.
/// This is used by unit tests; production code uses `render_all`.
#[cfg(test)]
fn render_one(
    content: &str,
    context: &tera::Context,
    path: &Path,
    tool_names: &[String],
) -> Result<String> {
    let mut tera = Tera::default();
    tera.register_function("tool", make_tool_function(tool_names.to_vec()));
    make_once_function(&mut tera);
    let template_name = path.to_string_lossy();
    tera.add_raw_template(&template_name, content)
        .with_context(|| format!("failed to parse template {}", path.display()))?;
    tera.render(&template_name, context)
        .with_context(|| format!("failed to render template {}", path.display()))
}

/// Creates a Tera function `once(name="...")` for include-once semantics.
///
/// Returns `true` the first time a given name is seen within a render pass, and
/// `false` on subsequent calls with the same name.  Templates wrap their content
/// in `{% if once(name="...") %} ... {% endif %}` so that a partial included
/// from multiple places expands at most once per output file.
///
/// Returns a shared handle so the caller can `.clear()` it between render
/// passes (each output file should deduplicate independently).
fn make_once_function(tera: &mut Tera) -> Arc<Mutex<HashSet<String>>> {
    let seen = Arc::new(Mutex::new(HashSet::new()));
    let seen_clone = Arc::clone(&seen);
    tera.register_function(
        "once",
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| tera::Error::msg("once() requires a `name` argument"))?;
            let first = seen_clone.lock().unwrap().insert(name.to_string());
            Ok(tera::Value::Bool(first))
        },
    );
    seen
}

/// Creates a Tera function `tool(name="...")` that validates the tool name exists
/// and returns it as-is.
fn make_tool_function(tool_names: Vec<String>) -> impl tera::Function {
    move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tera::Error::msg("tool() requires a `name` argument"))?;
        if !tool_names.iter().any(|t| t == name) {
            return Err(tera::Error::msg(format!(
                "unknown tool `{name}`, known tools: {}",
                tool_names.join(", ")
            )));
        }
        Ok(tera::Value::String(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plugin::PluginArgs, Platform};

    #[test]
    fn test_render_plain_content() {
        let context = tera::Context::new();
        let result = render_one("Hello world", &context, Path::new("test.md"), &[]).unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_render_with_variable() {
        let mut context = tera::Context::new();
        context.insert("platform", "claude");
        let result = render_one(
            "Platform is {{ platform }}",
            &context,
            Path::new("test.md"),
            &[],
        )
        .unwrap();
        assert_eq!(result, "Platform is claude");
    }

    #[test]
    fn test_render_unknown_tool_fails() {
        let context = tera::Context::new();
        let result = render_one(
            "{{ tool(name=\"no_such_tool\") }}",
            &context,
            Path::new("test.md"),
            &["move_package_status".to_string()],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_render_all_from_crate_root() {
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let global = crate::GlobalOpts {
            platform: Platform::Claude,
            content_dir: Some(content_root.clone()),
        };
        let mut context = tera::Context::from_serialize(&global).unwrap();
        context.insert("platform_display", global.platform.display_name());
        context.insert("flow_version", env!("CARGO_PKG_VERSION"));
        let args = PluginArgs {
            output_dir: PathBuf::from("."),
            initial_verification_timeout: 10,
            max_verification_timeout: 20,
            default_verification_attempts: 3,
            log: None,
        };
        context.insert("args", &args);

        let tool_names = crate::mcp::session::FlowSession::tool_names();
        let files = render_all(&content_root, &context, &tool_names).unwrap();
        assert!(!files.is_empty(), "should discover at least one file");

        let paths: Vec<_> = files.iter().map(|(p, _)| p.clone()).collect();
        assert!(
            paths.iter().any(|p| p.starts_with("skills")),
            "should find files under skills/"
        );

        // Verify that templates/ partials are NOT in the output.
        assert!(
            !paths.iter().any(|p| p.starts_with("templates")),
            "templates/ partials should not appear in output"
        );
    }
}
