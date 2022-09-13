// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// List of domains connected to an account.
// We can expand this so that we have fine-grained permissions in the future
type Domains = string[];

// Map of permissions associated to each account
export type Permissions = { [address: string]: Domains };
