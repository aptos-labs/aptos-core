// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod output;
mod render;

use crate::GlobalOpts;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

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
}

/// Generate plugin files for the given tool target.
pub fn run(args: &PluginArgs, global: &GlobalOpts) -> Result<()> {
    let content_root = match &global.content_dir {
        Some(dir) => dir.clone(),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    };

    let mut context =
        tera::Context::from_serialize(global).context("failed to build template context")?;
    context.insert("args", args);
    context.insert("platform_display", global.platform.display_name());
    context.insert("flow_version", env!("CARGO_PKG_VERSION"));

    let tool_names = crate::mcp::session::FlowSession::tool_names();
    let mut files = render::render_all(&content_root, &context, &tool_names)?;

    // Generate .mcp.json so Claude Code discovers the move-flow MCP server.
    // We launch through `sh -c` to split MOVE_FLOW_ARGS into argv tokens.
    let exec_cmd = match &args.log {
        Some(log_path) => format!(
            "set -f; set -- ${{MOVE_FLOW_ARGS:-mcp}}; exec \"${{MOVE_FLOW:-move-flow}}\" \"$@\" 2>>{}",
            log_path.display()
        ),
        None => "set -f; set -- ${MOVE_FLOW_ARGS:-mcp}; exec \"${MOVE_FLOW:-move-flow}\" \"$@\"".to_string(),
    };
    let mcp_config = serde_json::json!({
        "mcpServers": {
            "move-flow": {
                "command": "sh",
                "args": ["-c", exec_cmd]
            }
        }
    });
    files.push((
        PathBuf::from(".mcp.json"),
        serde_json::to_string_pretty(&mcp_config).context("failed to serialize .mcp.json")?,
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
        };
        let args = PluginArgs {
            output_dir: output_dir.path().to_path_buf(),
            initial_verification_timeout: 5,
            max_verification_timeout: 10,
            default_verification_attempts: 3,
            log: None,
        };

        run(&args, &global).expect("generate should succeed");

        // Verify some expected files exist
        assert!(output_dir.path().join("agents/move-verify.md").exists());
        assert!(output_dir.path().join("agents/move-inf.md").exists());
        assert!(output_dir.path().join("agents/move-inf-v2.md").exists());
        assert!(!output_dir.path().join("agents/move-dev.md").exists());
        assert!(output_dir.path().join("skills/move/SKILL.md").exists());
        assert!(output_dir
            .path()
            .join("skills/move-check/SKILL.md")
            .exists());
        assert!(output_dir
            .path()
            .join("skills/move-prove/SKILL.md")
            .exists());
        assert!(output_dir
            .path()
            .join("skills/move-inf-v2/SKILL.md")
            .exists());
        assert!(output_dir.path().join("hooks/hooks.json").exists());

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

        // Verify the move skill contains language reference content
        let skill_content =
            std::fs::read_to_string(output_dir.path().join("skills/move/SKILL.md")).unwrap();
        assert!(
            skill_content.contains("Move Language"),
            "expected move skill to contain language reference"
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
    }
}
