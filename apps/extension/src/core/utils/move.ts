// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable no-bitwise */

// See `explain_vm_status` at https://github.com/aptos-labs/aptos-core/blob/main/api/types/src/convert.rs

export enum MoveExecutionStatus {
  Success,
  OutOfGas,
  MoveAbort,
  ExecutionFailure,
  MiscellaneousError,
}

export function parseMoveVmStatus(status: string) {
  if (status === 'Executed successfully') {
    return MoveExecutionStatus.Success;
  }
  if (status === 'Out of gas') {
    return MoveExecutionStatus.OutOfGas;
  }
  if (status.startsWith('Move abort')) {
    return MoveExecutionStatus.MoveAbort;
  }
  if (status.startsWith('Execution failed')) {
    return MoveExecutionStatus.ExecutionFailure;
  }
  return MoveExecutionStatus.MiscellaneousError;
}

// See https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/move-stdlib/sources/error.move

export interface MoveErrorCode {
  category: MoveErrorCategory,
  reason: number,
}

export enum MoveErrorCategory {
  InvalidArgument = 0x1,
  OutOfRange,
  InvalidState,
  Unauthenticated,
  PermissionDenied,
  NotFound,
  Aborted,
  AlreadyExists,
  ResourceExhausted,
  Cancelled,
  Internal,
  NotImplemented,
  Unavailable,
}

const moveAbortPattern = /^Move abort: code (\d+)(?: at (\d+)::(.+))?$/;

export function parseMoveAbort(error: string): MoveErrorCode | null {
  const match = error.match(moveAbortPattern);
  if (!match) {
    return null;
  }

  const code = Number(match[1]);
  const category = (code & 0xff0000) >> 16;
  const reason = (code & 0xffff);

  return { category, reason };
}
