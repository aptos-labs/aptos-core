// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Move Package Cache
//!
//! The Package Cache is a component of the Move package system, responsible for downloading,
//! indexing, and storing all remote dependencies (Git repositories and on-chain packages) in a local,
//! globally accessible directory.
//!
//! It is implemented as a modular library and is intended to be used by other tools in the
//! Move ecosystem, such as the package resolver.
//!
//! ## Storage Layout
//!
//! The package cache manages two types of dependencies:
//! - **Git repositories** (`git/`): Full Git repositories and immutable checkouts at specific commits
//! - **On-chain packages** (`on-chain/`): Bytecode packages from the blockchain
//!
//! ```plaintext
//! .
//! ├── git
//! │   ├── checkouts/     # Immutable snapshots at specific commits
//! │   └── repos/         # Full Git repositories
//! └── on-chain/          # Fetched bytecode packages
//! ```
//!
//! ## Concurrency & Safety
//!
//! The package cache is designed to allow safe concurrent access:
//! - **File-based locking** for each repository, checkout, and package
//! - **Atomic directory renaming** to prevent visibility of incomplete data
//! - **Async operations** for high concurrency and non-blocking I/O
//!
//! ## Event Notifications
//!
//! Listener APIs are provided to allow clients to subscribe to events for progress tracking,
//! such as file lock acquisition, repository cloning and package downloads.
//
//! This is particularly useful for CLI tools to provide visual feedback and reassure users
//! that the tool is still actively progressing.

mod canonical;
mod file_lock;
mod listener;
mod package_cache;

pub use canonical::{CanonicalGitIdentity, CanonicalNodeIdentity};
pub use listener::{DebugPackageCacheListener, EmptyPackageCacheListener, PackageCacheListener};
pub use package_cache::PackageCache;
