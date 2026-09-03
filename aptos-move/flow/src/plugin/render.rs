// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{evaluation::EvaluationConfig, mcp::session::FlowSession};
use anyhow::{Context, Result};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
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
/// `omitted_skills` names skill directories to leave out of the output. A skill
/// whose tool is not served would fail `tool()` validation anyway, and shipping
/// instructions for a tool the session does not have is worse than not
/// shipping them.
pub fn render_all(
    content_root: &Path,
    context: &tera::Context,
    tool_names: &[String],
    omitted_skills: &[&str],
) -> Result<Vec<(PathBuf, String)>> {
    let mut tera = Tera::default();
    tera.register_function("tool", make_tool_function(tool_names.to_vec()));
    let once_seen = make_once_function(&mut tera);
    let frontmatter_seen = make_frontmatter_function(&mut tera);

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
            if out_prefix != "templates"
                && !omitted_skills
                    .iter()
                    .any(|skill| rel_within.starts_with(skill) && out_prefix == "skills")
            {
                output_names.push((out_path, template_name));
            }
        }
    }

    // Second pass: render output-producing templates.
    let mut results = Vec::new();
    for (out_path, template_name) in output_names {
        // Reset per-file state so each output file deduplicates independently.
        once_seen.lock().unwrap().clear();
        frontmatter_seen.store(false, Ordering::Relaxed);
        let rendered = tera
            .render(&template_name, context)
            .with_context(|| format!("failed to render template {}", out_path.display()))?;
        let rendered = if out_path.extension().is_some_and(|ext| ext == "md") {
            normalize_markdown_spacing(&rendered)
        } else {
            rendered
        };

        // Skills and agents must call frontmatter() to declare their metadata.
        let needs_frontmatter = out_path.starts_with("skills") || out_path.starts_with("agents");
        if needs_frontmatter {
            anyhow::ensure!(
                frontmatter_seen.load(Ordering::Relaxed),
                "{} does not call frontmatter(name=..., description=...)",
                out_path.display()
            );
            // Calling frontmatter() is not enough: the block only parses at the
            // very start of the file. Anything emitted ahead of it -- a newline
            // from a leading Tera tag, say -- leaves the file without metadata,
            // and the platform then skips the skill or agent without an error.
            anyhow::ensure!(
                rendered.starts_with("---\n"),
                "{} does not begin with its frontmatter block; \
                 the first line is {:?}. Frontmatter parses only at line 1, so \
                 this file would be silently ignored.",
                out_path.display(),
                rendered.lines().next().unwrap_or_default()
            );
        }

        results.push((out_path, rendered));
    }

    Ok(results)
}

/// Collapse redundant blank lines introduced by nested Tera includes while
/// preserving whitespace inside fenced examples.
fn normalize_markdown_spacing(content: &str) -> String {
    let mut output = String::with_capacity(content.len());
    let mut open_fence: Option<(char, usize)> = None;
    // Start-of-document counts as blank, so a leading blank line is dropped as
    // the redundant whitespace it is. A template whose first line is a Tera
    // control tag (`{% if ... %}`) emits one, which would push a skill's YAML
    // frontmatter off line 1 and make the whole skill fail to register.
    let mut previous_blank = true;

    for line in content.lines() {
        let fence = markdown_fence(line);
        let inside_fence = open_fence.is_some();
        let blank = line.trim().is_empty();

        if inside_fence || !blank || !previous_blank {
            writeln!(output, "{line}").unwrap();
        }
        previous_blank = !inside_fence && blank;

        match (open_fence, fence) {
            (None, Some((marker, length, _))) => open_fence = Some((marker, length)),
            (Some((open_marker, open_length)), Some((marker, length, only_marker)))
                if marker == open_marker && length >= open_length && only_marker =>
            {
                open_fence = None;
                previous_blank = false;
            },
            _ => {},
        }
    }

    output
}

/// Return the marker, run length, and whether nothing follows a Markdown fence.
fn markdown_fence(line: &str) -> Option<(char, usize, bool)> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let length = trimmed.chars().take_while(|ch| *ch == marker).count();
    (length >= 3).then(|| {
        let suffix = &trimmed[length..];
        (marker, length, suffix.trim().is_empty())
    })
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
    make_frontmatter_function(&mut tera);
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

/// Creates a Tera function `frontmatter(name="...", description="...")` that
/// renders YAML frontmatter and validates that both fields are non-empty. An
/// optional `argument_hint` is emitted as the skill's `argument-hint`.
///
/// Returns a shared flag that is set during rendering. The caller resets it
/// between render passes and checks that skill/agent files have called this
/// function exactly once.
fn make_frontmatter_function(tera: &mut Tera) -> Arc<AtomicBool> {
    let seen = Arc::new(AtomicBool::new(false));
    let seen_clone = Arc::clone(&seen);
    tera.register_function(
        "frontmatter",
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            if seen_clone.swap(true, Ordering::Relaxed) {
                return Err(tera::Error::msg(
                    "frontmatter() must be called exactly once per file",
                ));
            }
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| tera::Error::msg("frontmatter() requires a `name` argument"))?;
            if name.is_empty() {
                return Err(tera::Error::msg("frontmatter() `name` must not be empty"));
            }
            let description = args
                .get("description")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    tera::Error::msg("frontmatter() requires a `description` argument")
                })?;
            if description.is_empty() {
                return Err(tera::Error::msg(
                    "frontmatter() `description` must not be empty",
                ));
            }
            let argument_hint = match args.get("argument_hint").and_then(|v| v.as_str()) {
                Some(hint) if !hint.is_empty() => format!("\nargument-hint: {hint}"),
                _ => String::new(),
            };
            Ok(tera::Value::String(format!(
                "---\nname: {name}\ndescription: {description}{argument_hint}\n---"
            )))
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

/// Extract (name, description) from frontmatter in rendered content.
/// The format is generated by our `frontmatter()` Tera function.
fn parse_frontmatter(content: &str) -> Option<(String, String)> {
    let content = content.strip_prefix("---\n")?;
    let end = content.find("\n---")?;
    let block = &content[..end];
    let mut name = None;
    let mut desc = None;
    for line in block.lines() {
        if let Some(val) = line.strip_prefix("name: ") {
            name = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("description: ") {
            desc = Some(val.trim().to_string());
        }
    }
    Some((name?, desc?))
}

/// Generate a plugin README from the rendered files and MCP tool metadata.
pub fn generate_readme(
    files: &[(PathBuf, String)],
    platform_display: &str,
    evaluation: EvaluationConfig,
) -> String {
    let version = env!("CARGO_PKG_VERSION");
    let mut out = String::new();

    // Header
    writeln!(out, "# MoveFlow").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "Move smart contract development plugin for {platform_display}."
    )
    .unwrap();
    writeln!(out).unwrap();

    // Overview
    writeln!(out, "## Overview").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "MoveFlow provides skills, agents, hooks, and an MCP server for developing, \
         testing, and formally verifying [Move](https://aptos.dev/en/build/smart-contracts) \
         smart contracts on [Aptos](https://aptos.dev). Version {version}."
    )
    .unwrap();
    writeln!(out).unwrap();

    // Skills
    let mut skills: Vec<(String, String)> = files
        .iter()
        .filter(|(p, _)| p.starts_with("skills"))
        .filter_map(|(_, content)| parse_frontmatter(content))
        .collect();
    skills.sort();
    if !skills.is_empty() {
        writeln!(out, "## Skills").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "| Skill | Description |").unwrap();
        writeln!(out, "|-------|-------------|").unwrap();
        for (name, desc) in &skills {
            writeln!(out, "| `/{name}` | {desc} |").unwrap();
        }
        writeln!(out).unwrap();
    }

    // Agents
    let mut agents: Vec<(String, String)> = files
        .iter()
        .filter(|(p, _)| p.starts_with("agents"))
        .filter_map(|(_, content)| parse_frontmatter(content))
        .collect();
    agents.sort();
    if !agents.is_empty() {
        writeln!(out, "## Agents").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "| Agent | Description |").unwrap();
        writeln!(out, "|-------|-------------|").unwrap();
        for (name, desc) in &agents {
            writeln!(out, "| `{name}` | {desc} |").unwrap();
        }
        writeln!(out).unwrap();
    }

    // MCP Tools
    let tools = FlowSession::tool_descriptions(evaluation);
    if !tools.is_empty() {
        writeln!(out, "## MCP Tools").unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "Provided by the `move-flow` MCP server (configured in `.mcp.json`)."
        )
        .unwrap();
        writeln!(out).unwrap();
        writeln!(out, "| Tool | Description |").unwrap();
        writeln!(out, "|------|-------------|").unwrap();
        for (name, desc) in &tools {
            // Use only the first sentence for the table.
            let short = desc.split_once(". ").map_or(desc.as_str(), |(s, _)| s);
            writeln!(out, "| `{name}` | {short} |").unwrap();
        }
        writeln!(out).unwrap();
    }

    // Hooks
    let has_hooks = files.iter().any(|(p, _)| p.starts_with("hooks"));
    if has_hooks {
        writeln!(out, "## Hooks").unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "- **PostToolUse** — After every `Edit` or `Write` that touches a `.move` file, \
             runs syntax checks and auto-formatting."
        )
        .unwrap();
        writeln!(
            out,
            "- **UserPromptSubmit** — Detects Move package paths in the working directory."
        )
        .unwrap();
        writeln!(
            out,
            "- **SessionStart** — Verifies that the `move-flow` binary is installed."
        )
        .unwrap();
        writeln!(out).unwrap();
    }

    // Requirements
    writeln!(out, "## Requirements").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "- `move-flow` binary on `$PATH` (or set `$MOVE_FLOW` to the binary path)"
    )
    .unwrap();
    writeln!(out).unwrap();

    // Author
    writeln!(out, "## Author").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "Aptos Labs — version {version}").unwrap();

    out
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
    fn test_normalize_markdown_spacing_preserves_fenced_whitespace() {
        let input = "first\n\n\nsecond\n```move\nline\n\n\nline\n```\n\n\nlast\n";
        let expected = "first\n\nsecond\n```move\nline\n\n\nline\n```\n\nlast\n";
        assert_eq!(normalize_markdown_spacing(input), expected);
    }

    #[test]
    fn test_normalize_markdown_spacing_drops_leading_blank_lines() {
        // A template whose first line is a Tera control tag renders a leading
        // newline. Frontmatter parses only at line 1, so keeping it makes the
        // skill unregistrable -- and nothing reports an error.
        assert_eq!(
            normalize_markdown_spacing("\n---\nname: x\n"),
            "---\nname: x\n"
        );
        assert_eq!(
            normalize_markdown_spacing("\n\n\n---\nname: x\n"),
            "---\nname: x\n"
        );
        // Content that already starts correctly is untouched.
        assert_eq!(
            normalize_markdown_spacing("---\nname: x\n"),
            "---\nname: x\n"
        );
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
            inference_tactic: None,
            evaluation_mode: false,
            feedback_level: None,
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
            telemetry_jsonl: None,
            flow_source_commit: None,
        };
        context.insert("args", &args);
        context.insert("inference_tactic", "hybrid_guided");
        context.insert("wp_tool_enabled", &true);
        context.insert("guided_workflow", &true);
        context.insert("tactic_selectable", &true);
        context.insert("evaluation_mode", &false);
        context.insert("hook_env_setup", "");

        let tool_names = FlowSession::tool_names(EvaluationConfig {
            inference_tactic: crate::evaluation::InferenceTactic::HybridGuided,
            evaluation_mode: false,
            feedback_level: crate::evaluation::FeedbackLevel::Acceptance,
        });
        let files = render_all(&content_root, &context, &tool_names, &[]).unwrap();
        assert!(!files.is_empty(), "should discover at least one file");

        let paths: Vec<_> = files.iter().map(|(p, _)| p.clone()).collect();
        // Verify all expected plugin directories are represented in output.
        for dir in &["skills", "agents", "hooks"] {
            assert!(
                paths.iter().any(|p| p.starts_with(dir)),
                "should find files under {dir}/"
            );
        }

        // Every skill and agent must lead with its frontmatter block. The
        // platform parses it only at line 1 and otherwise skips the file in
        // silence, so a stray leading newline removes a whole skill without
        // any build or runtime error. `render_all` enforces this, and this
        // case pins it against the real content tree.
        for (path, body) in &files {
            if path.starts_with("skills") || path.starts_with("agents") {
                assert!(
                    body.starts_with("---\n"),
                    "{} must begin with frontmatter, found {:?}",
                    path.display(),
                    body.lines().next().unwrap_or_default()
                );
            }
        }

        // Verify that templates/ partials are NOT in the output.
        assert!(
            !paths.iter().any(|p| p.starts_with("templates")),
            "templates/ partials should not appear in output"
        );

        // Verify no output file is empty.
        for (path, content) in &files {
            assert!(
                !content.trim().is_empty(),
                "{} rendered to empty content",
                path.display()
            );
        }
    }
}
