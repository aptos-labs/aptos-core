// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PermissionApproval } from './approval';

export enum PermissionResponseStatus {
  Approved = 'approved',
  Rejected = 'rejected',
  Timeout = 'timeout',
}

export interface PermissionResponse {
  approval?: PermissionApproval;
  id: number;
  status: PermissionResponseStatus
}

export function isPermissionResponse(response: PermissionResponse): response is PermissionResponse {
  return response.id !== undefined
    && Object.values(PermissionResponseStatus).includes(response.status);
}
