// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod algo;

pub type StacklessBlockIdentifier = usize;
pub type StacklessBlockContent = algo::blocks_stackless::StacklessBlockContent;

pub mod datastructs;
pub mod stackless;
pub mod metadata;
