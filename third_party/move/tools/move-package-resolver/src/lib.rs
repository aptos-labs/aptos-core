// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Move Package Resolver
//!
//! This crate implements the Move Package Resolver -- component that is responsible for
//! traversing user-specified package manifests and constructing a fully resolved,
//! reproducible dependency graph. During this process, all remote dependencies are fetched
//! to the local file system for later use.
//!
//! ## High-level Architecture
//!
//! The resolver architecture centers around three main components:
//!
//! - **Package Cache** (external crate): Fetches and stores remote packages locally
//! - **Package Lock**: Records resolved identities of git branches and blockchain versions
//! - **Resolver**: Constructs the complete resolution graph and manages the `Move.lock` file
//!
//! ## Resolution Graph
//!
//! The resolver constructs a directed graph where:
//!
//! - **Nodes represent unique packages** identified by `(name, source_location)`
//!   - **Source locations** can be:
//!     - **Local**: Path to directory containing the package
//!     - **Git**: Repository URL + commit + subdirectory
//!     - **On-chain**: Network + address pair
//! - **Edges represent dependencies** with optional metadata for address renaming.

mod graph;
mod identity;
mod lock;
mod path;
mod resolver;

pub use graph::{graph_to_mermaid, Dependency, Package, ResolutionGraph};
pub use lock::PackageLock;
pub use resolver::resolve;
