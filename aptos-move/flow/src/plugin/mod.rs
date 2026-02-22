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
}

/// Generate plugin files for the given tool target.
pub fn run(args: &PluginArgs, global: &GlobalOpts) -> Result<()> {
    let content_root = match &global.content_dir {
        Some(dir) => dir.clone(),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    };

    let mut context =
        tera::Context::from_serialize(global).context("failed to build template context")?;
    context.insert("output_dir", &args.output_dir);
    context.insert("platform_display", global.platform.display_name());
    context.insert("flow_version", env!("CARGO_PKG_VERSION"));

    let tool_names = crate::mcp::session::FlowSession::tool_names();
    let mut files = render::render_all(&content_root, &context, &tool_names)?;

    // Generate .mcp.json so Claude Code discovers the move-flow MCP server.
    // We launch through `sh -c` to split MOVE_FLOW_ARGS into argv tokens.
    let mcp_config = serde_json::json!({
        "mcpServers": {
            "move-flow": {
                "command": "sh",
                "args": [
                    "-c",
                    "set -f; set -- ${MOVE_FLOW_ARGS:-mcp}; exec \"${MOVE_FLOW:-move-flow}\" \"$@\""
                ]
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
        };

        run(&args, &global).expect("generate should succeed");

        // Verify some expected files exist
        assert!(output_dir.path().join("agents/move-dev.md").exists());
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

        // Verify template expansion happened
        let agent_content =
            std::fs::read_to_string(output_dir.path().join("agents/move-dev.md")).unwrap();
        assert!(
            agent_content.contains("Claude Code"),
            "expected platform_display to be expanded"
        );

        // Verify the move skill contains tool usage instructions
        let skill_content =
            std::fs::read_to_string(output_dir.path().join("skills/move/SKILL.md")).unwrap();
        assert!(
            skill_content.contains("move_package_status"),
            "expected move skill to contain tool usage instructions"
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
