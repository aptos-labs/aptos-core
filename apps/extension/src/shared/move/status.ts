// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/**
 * @see `explain_vm_status` at https://github.com/aptos-labs/aptos-core/blob/main/api/types/src/convert.rs
 */
export enum MoveVmStatus {
  Success,
  OutOfGas,
  MoveAbort,
  ExecutionFailure,
  MiscellaneousError,
}

export function parseMoveVmStatus(status: string) {
  if (status === 'Executed successfully') {
    return MoveVmStatus.Success;
  }
  if (status === 'Out of gas') {
    return MoveVmStatus.OutOfGas;
  }
  if (status.startsWith('Move abort')) {
    return MoveVmStatus.MoveAbort;
  }
  if (status.startsWith('Execution failed')) {
    return MoveVmStatus.ExecutionFailure;
  }
  return MoveVmStatus.MiscellaneousError;
}
