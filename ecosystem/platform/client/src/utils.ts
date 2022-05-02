// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/**
 * Helper function for exhaustiveness checks.
 *
 * Hint: If this function is causing a type error, check to make sure that your
 * switch statement covers all cases!
 */
export function assertNever(x: never): never {
  throw new Error("Unexpected object: " + x);
}

export function randomUUID(): string {
  return crypto.randomUUID();
}
