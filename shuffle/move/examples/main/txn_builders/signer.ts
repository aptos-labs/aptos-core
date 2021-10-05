// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import { createHash } from "https://deno.land/std@0.77.0/hash/mod.ts";

export function hashPrefix(name: string): Uint8Array {
  const hash = createHash("sha3-256");
  hash.update("DIEM::");
  hash.update(name);
  return new Uint8Array(hash.digest());
}
