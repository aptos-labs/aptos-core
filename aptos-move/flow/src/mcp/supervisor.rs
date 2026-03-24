// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Crash-resilient supervisor for the MCP server.
//!
//! MCP runs over stdin/stdout, so a panic kills the connection and the client
//! may have no way to recover. To handle this, `move-flow mcp` runs as two
//! processes: a **supervisor** that re-execs the same binary as a **child**.
//!
//! The child inherits stdin/stderr (talking directly to the client) while
//! stdout is piped through the supervisor so it can inject an error
//! when a crash is detected. On a clean exit the supervisor exits too; on a
//! crash the child is respawned up to [`MAX_CRASHES`] times.
//!
//! The two processes distinguish themselves via [`RESTART_ENV_VAR`]: absent in
//! the supervisor, `"0"` on first child launch, `"1"` on restarts after a crash.

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::{
    io::{self, AsyncWriteExt},
    process::Command,
};

/// Environment variable used to distinguish the supervisor from its child.
/// See module docs for the protocol.
pub const RESTART_ENV_VAR: &str = "__MOVE_FLOW_RESTART";

const MAX_CRASHES: u32 = 10;
const CRASH_RESTARTING: &[u8] =
    b"{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32603,\"message\":\"MCP server crashed and is restarting. The previous tool call was lost - please retry it.\"}}\n";
const CRASH_GIVING_UP: &[u8] =
    b"{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32603,\"message\":\"MCP server crashed repeatedly and will not restart. Please restart the session.\"}}\n";

/// Entry point for the supervisor process. See module docs for details.
pub async fn run_supervised() -> Result<()> {
    move_compiler_v2::logging::setup_logging(None);

    let exe = std::env::current_exe().context("failed to determine current executable")?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut crashes = 0;

    loop {
        let mut child = Command::new(&exe)
            .args(&args)
            .env(RESTART_ENV_VAR, if crashes == 0 { "0" } else { "1" })
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped()) // piped so we can inject crash errors
            .stderr(Stdio::inherit())
            .spawn()
            .context("failed to spawn MCP server")?;

        let mut child_stdout = child.stdout.take().unwrap();

        let from_child = tokio::spawn(async move {
            let _ = io::copy(&mut child_stdout, &mut io::stdout()).await;
        });

        let status = child.wait().await?;
        // Child stdout is closed, so from_child will drain and finish.
        let _ = from_child.await;

        if status.success() {
            return Ok(());
        }

        // Only restart when the child was killed by a signal (e.g. SIGSEGV,
        // SIGABRT from a panic). Normal non-zero exits (bad config, permission
        // errors) should propagate immediately rather than looping.
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if status.signal().is_none() {
                // Not signal-killed — propagate the exit code.
                std::process::exit(status.code().unwrap_or(1));
            }
        }

        crashes += 1;
        if crashes >= MAX_CRASHES {
            log::error!("MCP server crashed {MAX_CRASHES} times, giving up");
            io::stdout().write_all(CRASH_GIVING_UP).await?;
            io::stdout().flush().await?;
            return Ok(());
        }
        log::error!("MCP server crashed ({crashes}/{MAX_CRASHES}), restarting");
        io::stdout().write_all(CRASH_RESTARTING).await?;
        io::stdout().flush().await?;
    }
}
