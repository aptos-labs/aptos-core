// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { ApiError, Types } from 'aptos';
import {
  MoveStatusCode,
  MoveVmError,
  MoveVmStatus,
  parseMoveMiscError,
  parseMoveVmStatus,
  parseMoveVmError,
} from 'shared/move';

/**
 * Handle vm errors returned in vm_status.
 * Should only happen for simulations
 * @param txn user transaction to check
 */
export function throwForVmError(txn: Types.UserTransaction) {
  const vmStatus = parseMoveVmStatus(txn.vm_status);

  if (vmStatus === MoveVmStatus.MiscellaneousError) {
    const statusCodeKey = parseMoveMiscError(txn.vm_status);
    throw new MoveVmError(statusCodeKey);
  }

  if (vmStatus === MoveVmStatus.OutOfGas) {
    throw new MoveVmError('OUT_OF_GAS');
  }

  if (vmStatus === MoveVmStatus.ExecutionFailure) {
    throw new MoveVmError();
  }
}

/**
 * Map an ApiError from the Aptos SDK into a catchable MoveVmError
 * @param err error to handle
 */
export function handleApiError(err: any) {
  if (err instanceof ApiError) {
    const { message } = JSON.parse(err.message);
    const statusCode = err.vmErrorCode !== undefined
      ? Number(err.vmErrorCode) as MoveStatusCode
      : undefined;
    const statusCodeKey = parseMoveVmError(message);
    throw new MoveVmError(statusCodeKey, statusCode);
  }
}
