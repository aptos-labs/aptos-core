// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  DappInfo,
  Permission,
  PermissionApproval,
  PermissionResponse,
  PermissionResponseStatus,
} from '../types';

export const PROMPT_SIZE = { height: 520, width: 375 };
export const PROMPT_PATHNAME = 'prompt.html';

export type PermissionResponseErrorStatus =
  PermissionResponseStatus.Rejected | PermissionResponseStatus.Timeout;

export class PermissionResponseError extends Error {
  constructor(readonly status: PermissionResponseErrorStatus) {
    const keyIndex = Object.values(PermissionResponseStatus).indexOf(status);
    const statusMessage = Object.keys(PermissionResponseStatus).at(keyIndex);
    super(statusMessage);
    this.name = 'PermissionResponseError';
    Object.setPrototypeOf(this, PermissionResponseError.prototype);
  }
}

export function handlePermissionResponse(response: PermissionResponse) {
  switch (response.status) {
    case PermissionResponseStatus.Approved:
      return response.approval;
    default:
      throw new PermissionResponseError(response.status);
  }
}

export interface PermissionHandler {
  requestPermission(dappInfo: DappInfo, request: Permission): Promise<PermissionApproval>;
  sendPermissionResponse(response: PermissionResponse): Promise<void>;
}
