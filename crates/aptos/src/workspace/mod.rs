// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// The `WorkspaceCommand` struct lives in `aptos-workspace-server` and is used
// directly in the CLI dispatch (`lib.rs`). No `CliCommand` impl is needed here
// because `lib.rs` calls `workspace.run()` directly rather than going through
// the `CliCommand` trait dispatch.
