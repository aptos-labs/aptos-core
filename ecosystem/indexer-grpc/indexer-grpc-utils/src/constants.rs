// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// The maximum number of transactions that can be stored in a blob.
pub const BLOB_STORAGE_SIZE: usize = 1_000;
/// GRPC request metadata key for the token ID.
pub const GRPC_AUTH_TOKEN_HEADER: &str = "x-aptos-data-authorization";
/// GRPC request metadata key for the request name. This is used to identify the
/// data destination.
pub const GRPC_REQUEST_NAME_HEADER: &str = "x-aptos-request-name";
// Limit the message size to 15MB. By default the downstream can receive up to 15MB.
pub const MESSAGE_SIZE_LIMIT: usize = 1024 * 1024 * 15;
