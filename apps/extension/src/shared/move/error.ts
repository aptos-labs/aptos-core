// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosError } from 'aptos/dist/generated';

/**
 * Move VM status codes that describe an error in the VM
 * @see https://github.com/move-language/move/blob/3f862abe908ab09710342f1b1cc79b8961ea8a1b/language/move-core/types/src/vm_status.rs#L418
 */
export enum MoveStatusCode {
  SEQUENCE_NUMBER_TOO_OLD = 3,
  INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE = 5,
  OUT_OF_GAS = 4002,
}

/**
 * Note: using key of status code as identifier, as simulations don't return a `vm_status_code`
 */
export type MoveStatusCodeKey = keyof typeof MoveStatusCode;

/**
 * Move misc error message pattern returned as transaction VM status
 * @see `explain_vm_status` at https://github.com/aptos-labs/aptos-core/blob/main/api/types/src/convert.rs
 */
const miscErrorPattern = /^Transaction Executed and Committed with Error (.+)$/;

/**
 * Move VM error pattern returned in an `AptosError`
 * @see {@link AptosError}
 * @see `create_internal` at https://github.com/aptos-labs/aptos-core/blob/main/api/src/transactions.rs
 */
const vmErrorPattern = /^Invalid transaction: Type: Validation Code: (.+)$/;

/**
 * Indicates a VM error
 */
export class MoveVmError extends Error {
  constructor(
    readonly statusCodeKey?: MoveStatusCodeKey,
    readonly statusCode = statusCodeKey && MoveStatusCode[statusCodeKey],
  ) {
    super();
    this.name = 'MoveVmError';
    Object.setPrototypeOf(this, MoveVmError.prototype);
    this.message = statusCodeKey ?? 'Generic error';
  }
}

/**
 * Parse status code from the VM status of a transaction
 * @param vmStatus status of a transaction
 */
export function parseMoveMiscError(vmStatus: string) {
  const match = vmStatus.match(miscErrorPattern);
  return match !== null
    ? match[1] as MoveStatusCodeKey
    : undefined;
}

/**
 * Parse status code from an `AptosError`
 * @param error error returned from the API
 */
export function parseMoveVmError(error: AptosError) {
  const match = error.message.match(vmErrorPattern);
  return match !== null
    ? match[1] as MoveStatusCodeKey
    : undefined;
}
