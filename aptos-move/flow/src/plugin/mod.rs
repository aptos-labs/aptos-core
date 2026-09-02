// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod output;
mod render;

use crate::{evaluation::sha256_hex, GlobalOpts};
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

/// A string escaped for use *inside* a JSON string literal, without the
/// surrounding quotes.
fn json_string_body(value: &str) -> String {
    let quoted = serde_json::Value::String(value.to_string()).to_string();
    quoted[1..quoted.len() - 1].to_string()
}

fn shell_single_quote(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(ch);
        }
    }
    quoted.push('\'');
    quoted
}

/// Arguments for the `plugin` subcommand.
#[derive(Parser, Debug, serde::Serialize)]
pub struct PluginArgs {
    /// Output directory for generated files.
    pub output_dir: PathBuf,

    /// Initial timeout (seconds) for verification runs.
    #[arg(long, default_value_t = 5)]
    pub initial_verification_timeout: u64,

    /// Maximum timeout (seconds) for verification runs.
    #[arg(long, default_value_t = 10)]
    pub max_verification_timeout: u64,

    /// Default number of verification attempts before giving up.
    #[arg(long, default_value_t = 2)]
    pub default_verification_attempts: u64,

    /// Log file for MCP server stderr. If not set, stderr is not redirected.
    #[arg(long)]
    pub log: Option<PathBuf>,

    /// JSONL telemetry path forwarded to the generated MCP server.
    #[arg(long)]
    pub telemetry_jsonl: Option<PathBuf>,

    /// Full aptos-core source commit recorded in evaluation manifests.
    #[arg(long)]
    pub flow_source_commit: Option<String>,
}

/// Generate plugin files for the given tool target.
pub fn run(args: &PluginArgs, global: &GlobalOpts) -> Result<()> {
    let content_root = match &global.content_dir {
        Some(dir) => dir.clone(),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    };

    let evaluation = global.evaluation_config()?;
    let flow_source_commit = args
        .flow_source_commit
        .clone()
        .or_else(|| std::env::var("GIT_SHA").ok())
        .unwrap_or_else(|| "unrecorded".to_string());
    if evaluation.evaluation_mode {
        anyhow::ensure!(
            flow_source_commit.len() == 40
                && flow_source_commit.chars().all(|ch| ch.is_ascii_hexdigit()),
            "evaluation plugins require --flow-source-commit with a full 40-hex commit"
        );
    }
    let mut context =
        tera::Context::from_serialize(global).context("failed to build template context")?;
    context.insert("args", args);
    context.insert("platform_display", global.platform.display_name());
    context.insert("flow_version", env!("CARGO_PKG_VERSION"));
    context.insert("inference_tactic", evaluation.inference_tactic.as_str());
    context.insert("wp_tool_enabled", &evaluation.wp_tool_enabled());
    context.insert("tactic_selectable", &evaluation.tactic_selectable());
    context.insert(
        "guided_workflow",
        &evaluation.inference_tactic.guided_workflow(),
    );
    context.insert("evaluation_mode", &evaluation.evaluation_mode);
    context.insert("feedback_level", evaluation.feedback_level.as_str());
    context.insert(
        "acceptance_check_enabled",
        &evaluation.acceptance_check_enabled(),
    );
    // The settings a launched process needs, named once. Hooks receive them as
    // shell exports and the MCP server as a JSON `env` block; a setting added
    // here reaches both.
    let evaluation_mode = if evaluation.evaluation_mode { "1" } else { "0" };
    let mut session_env: Vec<(&str, String)> = vec![
        (
            crate::evaluation::INFERENCE_TACTIC_ENV_VAR,
            evaluation.inference_tactic.as_str().to_string(),
        ),
        (
            crate::evaluation::EVALUATION_MODE_ENV_VAR,
            evaluation_mode.to_string(),
        ),
        (
            crate::evaluation::FEEDBACK_LEVEL_ENV_VAR,
            evaluation.feedback_level.as_str().to_string(),
        ),
    ];
    session_env.push((
        crate::evaluation::SOURCE_COMMIT_ENV_VAR,
        flow_source_commit.clone(),
    ));
    if let Some(path) = &args.telemetry_jsonl {
        session_env.push((
            crate::mcp::TELEMETRY_JSONL_ENV_VAR,
            path.to_string_lossy().into_owned(),
        ));
    }
    let hook_env_setup: String = session_env
        .iter()
        .map(|(name, value)| format!("export {name}={}; ", shell_single_quote(value)))
        .collect();
    // The value is interpolated into a JSON string literal in `hooks.json`,
    // so shell-quoting alone is not enough: an unescaped `"` would close that
    // string and let a second `command` property be injected into the hook.
    context.insert("hook_env_setup", &json_string_body(&hook_env_setup));

    let tool_names = crate::mcp::session::FlowSession::tool_names(evaluation);
    let mut sorted_tool_names = tool_names.clone();
    sorted_tool_names.sort();
    let tool_list_sha256 = sha256_hex(sorted_tool_names.join("\n").as_bytes());
    // A skill for a tool this plugin does not serve would name a tool that is
    // not there, so it is left out with the tool.
    let omitted_skills: Vec<&str> = if evaluation.replay_tool_enabled() {
        Vec::new()
    } else {
        vec!["move-replay"]
    };
    let mut files = render::render_all(&content_root, &context, &tool_names, &omitted_skills)?;

    // Generate .mcp.json so Claude Code discovers the move-flow MCP server.
    // We launch through `sh -c` to split MOVE_FLOW_ARGS into argv tokens.
    let exec_cmd = match &args.log {
        Some(log_path) => format!(
            "set -f; set -- ${{MOVE_FLOW_ARGS:-mcp}}; exec \"${{MOVE_FLOW:-move-flow}}\" \"$@\" 2>>{}",
            shell_single_quote(&log_path.to_string_lossy())
        ),
        None => "set -f; set -- ${MOVE_FLOW_ARGS:-mcp}; exec \"${MOVE_FLOW:-move-flow}\" \"$@\"".to_string(),
    };
    let mut mcp_env = serde_json::Map::new();
    for (name, value) in &session_env {
        mcp_env.insert(name.to_string(), serde_json::Value::from(value.clone()));
    }
    // The `EXPECTED_*` mirrors let the server refuse a session whose settings
    // drifted from the ones this manifest was generated with.
    for (name, value) in [
        (
            crate::evaluation::EXPECTED_INFERENCE_TACTIC_ENV_VAR,
            evaluation.inference_tactic.as_str().to_string(),
        ),
        (
            crate::evaluation::EXPECTED_EVALUATION_MODE_ENV_VAR,
            evaluation_mode.to_string(),
        ),
        (
            crate::evaluation::EXPECTED_FEEDBACK_LEVEL_ENV_VAR,
            evaluation.feedback_level.as_str().to_string(),
        ),
        (
            crate::evaluation::EXPECTED_TOOL_LIST_SHA256_ENV_VAR,
            tool_list_sha256.clone(),
        ),
    ] {
        mcp_env.insert(name.to_string(), serde_json::Value::from(value));
    }
    let mcp_config = serde_json::json!({
        "mcpServers": {
            "move-flow": {
                "command": "sh",
                "args": ["-c", exec_cmd],
                "env": mcp_env,
            }
        }
    });
    files.push((
        PathBuf::from(".mcp.json"),
        serde_json::to_string_pretty(&mcp_config).context("failed to serialize .mcp.json")?,
    ));

    // Claude Code plugin manifest (required by `claude plugin validate`).
    if global.platform == crate::Platform::Claude {
        let plugin_manifest = serde_json::json!({
            "name": "move-flow",
            "description": "Move smart contract development for Aptos",
            "version": env!("CARGO_PKG_VERSION"),
            "author": {
                "name": "Aptos Labs",
                "url": "https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/flow"
            }
        });
        files.push((
            PathBuf::from(".claude-plugin/plugin.json"),
            serde_json::to_string_pretty(&plugin_manifest)
                .context("failed to serialize plugin.json")?,
        ));
    }

    let rendered_skill = files
        .iter()
        .find(|(path, _)| path == &PathBuf::from("skills/move-inf/SKILL.md"))
        .map(|(_, content)| content.as_str())
        .context("generated plugin is missing skills/move-inf/SKILL.md")?;
    let generation_manifest = serde_json::json!({
        "schema_version": 1,
        "flow_version": env!("CARGO_PKG_VERSION"),
        "flow_source_commit": flow_source_commit,
        "inference_tactic": evaluation.inference_tactic.as_str(),
        "evaluation_mode": evaluation.evaluation_mode,
        "feedback_level": evaluation.feedback_level.as_str(),
        "rendered_inference_skill_sha256": sha256_hex(rendered_skill.as_bytes()),
        "mcp_tool_list_sha256": tool_list_sha256,
        "mcp_tools": sorted_tool_names,
    });
    files.push((
        PathBuf::from("move-flow-manifest.json"),
        serde_json::to_string_pretty(&generation_manifest)
            .context("failed to serialize move-flow-manifest.json")?,
    ));

    files.push((
        PathBuf::from("README.md"),
        render::generate_readme(&files, global.platform.display_name(), evaluation),
    ));

    output::write_output(&args.output_dir, &files)?;

    println!(
        "MoveFlow: generated {} file(s) for {} in {}",
        files.len(),
        global.platform.display_name(),
        args.output_dir.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Platform;
    use tempfile::TempDir;

    #[test]
    fn test_generate_claude() {
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let output_dir = TempDir::new().unwrap();

        let global = GlobalOpts {
            platform: Platform::Claude,
            content_dir: Some(content_root),
            inference_tactic: None,
            evaluation_mode: false,
            feedback_level: None,
        };
        let args = PluginArgs {
            output_dir: output_dir.path().to_path_buf(),
            initial_verification_timeout: 5,
            max_verification_timeout: 10,
            default_verification_attempts: 3,
            log: None,
            telemetry_jsonl: None,
            flow_source_commit: None,
        };

        run(&args, &global).expect("generate should succeed");

        // Verify some expected files exist
        assert!(output_dir.path().join("agents/move-verify.md").exists());
        assert!(output_dir.path().join("agents/move-inf.md").exists());
        assert!(output_dir.path().join("skills/move/SKILL.md").exists());
        assert!(output_dir
            .path()
            .join("skills/move-check/SKILL.md")
            .exists());
        assert!(output_dir
            .path()
            .join("skills/move-prove/SKILL.md")
            .exists());
        assert!(output_dir.path().join("hooks/hooks.json").exists());

        // Verify hooks.json is valid JSON with expected event names.
        let hooks_content =
            std::fs::read_to_string(output_dir.path().join("hooks/hooks.json")).unwrap();
        let hooks_json: serde_json::Value =
            serde_json::from_str(&hooks_content).expect("hooks.json must be valid JSON");
        let hooks_obj = hooks_json["hooks"]
            .as_object()
            .expect("hooks.json must contain a 'hooks' object");
        let valid_events = [
            "PreToolUse",
            "PostToolUse",
            "PostToolUseFailure",
            "PermissionRequest",
            "UserPromptSubmit",
            "Notification",
            "Stop",
            "SubagentStart",
            "SubagentStop",
            "SessionStart",
            "SessionEnd",
            "TeammateIdle",
            "TaskCompleted",
            "PreCompact",
            "ConfigChange",
            "WorktreeCreate",
            "WorktreeRemove",
            "InstructionsLoaded",
        ];
        let valid_hook_types = ["command", "prompt", "agent"];
        for key in hooks_obj.keys() {
            assert!(
                valid_events.contains(&key.as_str()),
                "hooks.json contains unknown event name: {key}"
            );
        }
        // Validate hook type values.
        for (event, entries) in hooks_obj {
            let entries = entries
                .as_array()
                .unwrap_or_else(|| panic!("hooks.json {event}: expected array"));
            for entry in entries {
                let inner = entry["hooks"]
                    .as_array()
                    .unwrap_or_else(|| panic!("hooks.json {event}: expected 'hooks' array"));
                for hook in inner {
                    let hook_type = hook["type"]
                        .as_str()
                        .unwrap_or_else(|| panic!("hooks.json {event}: hook missing 'type'"));
                    assert!(
                        valid_hook_types.contains(&hook_type),
                        "hooks.json {event}: unknown hook type: {hook_type}"
                    );
                }
            }
        }

        // Verify agent files were generated with correct names
        let verify_content =
            std::fs::read_to_string(output_dir.path().join("agents/move-verify.md")).unwrap();
        assert!(
            verify_content.contains("move-verify"),
            "expected verify agent file to contain its name"
        );
        let inf_content =
            std::fs::read_to_string(output_dir.path().join("agents/move-inf.md")).unwrap();
        assert!(
            inf_content.contains("move-inf"),
            "expected inf agent file to contain its name"
        );
        assert!(inf_content.contains("### Guided hybrid tactic"));
        assert!(inf_content.contains("Follow this order:"));
        assert!(inf_content.contains("**Run WP over the requested scope.**"));
        // One warned function at a time, reran under a function filter.
        assert!(inf_content.contains("**Take one warned function.**"));
        assert!(inf_content.contains("filter: \"module::function\""));
        assert!(inf_content.contains("## Final report"));
        // Outside an evaluation a hybrid plugin carries both hybrid tactics,
        // with the rendered one as the default an invocation may override.
        // The direct tactic is a separate plugin.
        assert!(inf_content.contains("### Tactic"));
        assert!(inf_content.contains("**hybrid-guided**"));
        assert!(inf_content.contains("### Flexible hybrid tactic"));
        assert!(!inf_content.contains("### Direct tactic"));
        let inf_skill =
            std::fs::read_to_string(output_dir.path().join("skills/move-inf/SKILL.md")).unwrap();
        assert!(inf_skill.contains("argument-hint: [hybrid-guided|hybrid-flexible] [scope]"));

        // Verify the move skill contains language reference content
        let skill_content =
            std::fs::read_to_string(output_dir.path().join("skills/move/SKILL.md")).unwrap();
        assert!(
            skill_content.contains("Move Language"),
            "expected move skill to contain language reference"
        );

        // Verify .claude-plugin/plugin.json manifest is generated
        let manifest_path = output_dir.path().join(".claude-plugin/plugin.json");
        assert!(manifest_path.exists(), "plugin.json should exist");
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest["name"], "move-flow");
        assert!(
            manifest["description"]
                .as_str()
                .is_some_and(|s| !s.is_empty()),
            "plugin.json should have a description"
        );
        assert!(
            manifest["version"].as_str().is_some_and(|s| !s.is_empty()),
            "plugin.json should have a version"
        );
        assert!(
            manifest["author"]["name"]
                .as_str()
                .is_some_and(|s| !s.is_empty()),
            "plugin.json should have an author name"
        );

        // Verify README.md is generated at the output root
        let readme_path = output_dir.path().join("README.md");
        assert!(
            readme_path.exists(),
            "README.md should exist at output root"
        );
        let readme = std::fs::read_to_string(&readme_path).unwrap();
        assert!(readme.contains("# MoveFlow"), "README should have title");
        assert!(readme.contains("## Skills"), "README should list skills");
        assert!(readme.contains("## Agents"), "README should list agents");
        assert!(
            readme.contains("## MCP Tools"),
            "README should list MCP tools"
        );
        assert!(readme.contains("## Hooks"), "README should list hooks");
        assert!(
            readme.contains("move_package_status"),
            "README should include MCP tool names"
        );

        // Verify .mcp.json is generated at the output root
        let mcp_path = output_dir.path().join(".mcp.json");
        assert!(mcp_path.exists(), ".mcp.json should exist at output root");
        let mcp_content = std::fs::read_to_string(&mcp_path).unwrap();
        let mcp_json: serde_json::Value = serde_json::from_str(&mcp_content).unwrap();
        let server_config = &mcp_json["mcpServers"]["move-flow"];
        let expected_args = serde_json::json!([
            "-c",
            "set -f; set -- ${MOVE_FLOW_ARGS:-mcp}; exec \"${MOVE_FLOW:-move-flow}\" \"$@\""
        ]);
        assert!(
            server_config.is_object(),
            ".mcp.json should contain move-flow server config"
        );
        assert_eq!(server_config["command"], "sh");
        assert_eq!(server_config["args"], expected_args);
        assert_eq!(
            server_config["env"]["MOVE_FLOW_INFERENCE_TACTIC"],
            "hybrid_guided"
        );
        assert_eq!(server_config["env"]["MOVE_FLOW_EVALUATION_MODE"], "0");
        assert_eq!(
            server_config["env"]["MOVE_FLOW_EXPECTED_INFERENCE_TACTIC"],
            "hybrid_guided"
        );

        let generation_manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(output_dir.path().join("move-flow-manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(generation_manifest["inference_tactic"], "hybrid_guided");
        assert_eq!(generation_manifest["evaluation_mode"], false);
        assert_eq!(
            server_config["env"]["MOVE_FLOW_EXPECTED_TOOL_LIST_SHA256"],
            generation_manifest["mcp_tool_list_sha256"]
        );
        assert_eq!(
            generation_manifest["rendered_inference_skill_sha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
    }

    #[test]
    fn test_generate_claude_with_log_path_shell_quoted() {
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let output_dir = TempDir::new().unwrap();
        let log_path = output_dir
            .path()
            .join("logs")
            .join("stderr $x `cmd` 'q'.log");

        let global = GlobalOpts {
            platform: Platform::Claude,
            content_dir: Some(content_root),
            inference_tactic: None,
            evaluation_mode: false,
            feedback_level: None,
        };
        let args = PluginArgs {
            output_dir: output_dir.path().to_path_buf(),
            initial_verification_timeout: 5,
            max_verification_timeout: 10,
            default_verification_attempts: 3,
            log: Some(log_path.clone()),
            telemetry_jsonl: None,
            flow_source_commit: None,
        };

        run(&args, &global).expect("generate should succeed");

        let mcp_content = std::fs::read_to_string(output_dir.path().join(".mcp.json")).unwrap();
        let mcp_json: serde_json::Value = serde_json::from_str(&mcp_content).unwrap();
        let server_config = &mcp_json["mcpServers"]["move-flow"];
        let expected_args = serde_json::json!([
            "-c",
            format!(
                "set -f; set -- ${{MOVE_FLOW_ARGS:-mcp}}; exec \"${{MOVE_FLOW:-move-flow}}\" \"$@\" 2>>{}",
                shell_single_quote(&log_path.to_string_lossy())
            )
        ]);

        assert_eq!(server_config["command"], "sh");
        assert_eq!(server_config["args"], expected_args);
    }

    #[test]
    fn test_json_string_body_escapes_quotes_and_backslashes() {
        // The value is interpolated into a JSON string in `hooks.json`; an
        // unescaped quote would close it and admit a second `command`.
        assert_eq!(json_string_body(r#"a"b"#), r#"a\"b"#);
        assert_eq!(json_string_body(r"a\b"), r"a\\b");
        assert_eq!(json_string_body("plain"), "plain");
    }

    #[test]
    fn test_shell_single_quote_escapes_single_quotes() {
        assert_eq!(shell_single_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn test_generate_agent_only_evaluation_plugin() {
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let output_dir = TempDir::new().unwrap();
        let telemetry_path = output_dir.path().join("telemetry/events.jsonl");
        let global = GlobalOpts {
            platform: Platform::Claude,
            content_dir: Some(content_root),
            inference_tactic: Some(crate::evaluation::InferenceTactic::AgentOnly),
            evaluation_mode: true,
            feedback_level: None,
        };
        let args = PluginArgs {
            output_dir: output_dir.path().to_path_buf(),
            initial_verification_timeout: 5,
            max_verification_timeout: 10,
            default_verification_attempts: 3,
            log: None,
            telemetry_jsonl: Some(telemetry_path.clone()),
            flow_source_commit: Some("6d836beedc56fc70c54f3b3046d1d248d850c64b".to_string()),
        };

        run(&args, &global).expect("agent-only generation should succeed");
        let skill =
            std::fs::read_to_string(output_dir.path().join("skills/move-inf/SKILL.md")).unwrap();
        assert!(skill.contains("### Direct tactic"));
        assert!(!skill.contains("move_package_wp"));
        assert!(!skill.contains("### Guided hybrid tactic"));
        assert!(!skill.contains("### Flexible hybrid tactic"));
        assert!(skill.contains("Never disable or skip verification"));
        assert!(skill.contains("### Loop abstractions"));
        assert!(skill.contains("## Final report"));
        // Shared WP concepts remain available as reasoning background, but
        // neither the tool nor its usage reference can leak into this arm.
        assert!(skill.contains("### Weakest-precondition reasoning"));
        assert!(!skill.contains("### WP tool"));
        // Every authored contract claims `pragma opaque`: a caller may be
        // verified against it without reading the body.
        assert!(skill.contains("Give the specification you author for the target `pragma opaque`"));

        let readme = std::fs::read_to_string(output_dir.path().join("README.md")).unwrap();
        assert!(!readme.contains("move_package_wp"));
        let mcp: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(output_dir.path().join(".mcp.json")).unwrap(),
        )
        .unwrap();
        let env = &mcp["mcpServers"]["move-flow"]["env"];
        assert_eq!(env["MOVE_FLOW_INFERENCE_TACTIC"], "agent_only");
        assert_eq!(env["MOVE_FLOW_EVALUATION_MODE"], "1");
        assert_eq!(env["MOVE_FLOW_EXPECTED_INFERENCE_TACTIC"], "agent_only");
        assert_eq!(env["MOVE_FLOW_EXPECTED_EVALUATION_MODE"], "1");
        assert_eq!(
            env["MOVE_FLOW_TELEMETRY_JSONL"],
            telemetry_path.to_string_lossy().as_ref()
        );
        let hooks = std::fs::read_to_string(output_dir.path().join("hooks/hooks.json")).unwrap();
        assert!(hooks.contains("MOVE_FLOW_INFERENCE_TACTIC='agent_only'"));
        assert!(hooks.contains("MOVE_FLOW_EVALUATION_MODE='1'"));
        assert!(hooks.contains(&format!(
            "MOVE_FLOW_TELEMETRY_JSONL={}",
            shell_single_quote(&telemetry_path.to_string_lossy())
        )));
    }

    #[test]
    fn test_generate_flexible_hybrid_plugin_without_ordered_inference_tasks() {
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let output_dir = TempDir::new().unwrap();
        let global = GlobalOpts {
            platform: Platform::Claude,
            content_dir: Some(content_root),
            inference_tactic: Some(crate::evaluation::InferenceTactic::HybridFlexible),
            evaluation_mode: true,
            feedback_level: None,
        };
        let args = PluginArgs {
            output_dir: output_dir.path().to_path_buf(),
            initial_verification_timeout: 5,
            max_verification_timeout: 10,
            default_verification_attempts: 3,
            log: None,
            telemetry_jsonl: None,
            flow_source_commit: Some("6d836beedc56fc70c54f3b3046d1d248d850c64b".to_string()),
        };

        run(&args, &global).expect("flexible generation should succeed");
        let skill =
            std::fs::read_to_string(output_dir.path().join("skills/move-inf/SKILL.md")).unwrap();
        assert!(skill.contains("### Flexible hybrid tactic"));
        assert!(skill.contains("move_package_wp"));
        assert!(skill.contains("available as an inference pass"));
        assert!(skill.contains("whether and when to use it"));
        // WP runs on any scope; the loop diagnostics are what guide the
        // invariant, so no arm is told to withhold the call.
        assert!(skill.contains("It runs on any scope, loops included"));
        assert!(!skill.contains("Do not call WP"));
        assert!(skill.contains("### WP tool"));
        assert!(skill.contains("Give the specification you author for the target `pragma opaque`"));
        assert!(skill.contains("### Loop abstractions"));
        assert!(skill.contains("## Final report"));
        // This arm gets capability knowledge, not the prescribed hybrid
        // workflow. Keeping this boundary explicit makes the guided contrast
        // interpretable.
        assert!(!skill.contains("### Guided hybrid tactic"));
        assert!(!skill.contains("Follow this order:"));
        assert!(!skill.contains("**Run WP.**"));
        assert!(!skill.contains("**Supply the invariants it asks for.**"));

        let guided_output_dir = TempDir::new().unwrap();
        let guided_global = GlobalOpts {
            inference_tactic: Some(crate::evaluation::InferenceTactic::HybridGuided),
            ..global
        };
        let guided_args = PluginArgs {
            output_dir: guided_output_dir.path().to_path_buf(),
            ..args
        };
        run(&guided_args, &guided_global).expect("guided generation should succeed");
        let guided_skill =
            std::fs::read_to_string(guided_output_dir.path().join("skills/move-inf/SKILL.md"))
                .unwrap();
        let shared_marker = "## Specification inference reference";
        let flexible_reference = skill
            .split_once(shared_marker)
            .expect("flexible skill should contain shared reference")
            .1;
        let guided_reference = guided_skill
            .split_once(shared_marker)
            .expect("guided skill should contain shared reference")
            .1;
        assert_eq!(flexible_reference, guided_reference);
    }
}
