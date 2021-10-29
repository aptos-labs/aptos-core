// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::vfs::VirtualFileSystem;
use lsp_server::Connection;

/// The context within which the language server is running.
pub struct Context {
    /// The connection with the language server's client.
    pub connection: Connection,
    /// The files that the language server is providing information about.
    pub files: VirtualFileSystem,
}
