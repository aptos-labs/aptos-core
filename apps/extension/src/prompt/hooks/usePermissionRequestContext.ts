// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import constate from 'constate';
import { useState } from 'react';
import {
  PermissionApproval,
  PermissionHandler,
  PermissionRequest,
  PermissionResponseStatus,
} from 'shared/permissions';

export interface ApprovalState {
  args?: PermissionApproval,
  canApprove: boolean,
}

export interface PermissionRequestProviderProps {
  permissionRequest: PermissionRequest,
}

export const [PermissionRequestContextProvider, usePermissionRequestContext] = constate(({
  permissionRequest,
}: PermissionRequestProviderProps) => {
  const [approvalState, setApprovalState] = useState<ApprovalState>({ canApprove: false });

  const approve = async () => {
    await PermissionHandler.sendPermissionResponse({
      approval: approvalState.args,
      id: permissionRequest.id,
      status: PermissionResponseStatus.Approved,
    });
  };

  const reject = async () => {
    await PermissionHandler.sendPermissionResponse({
      id: permissionRequest.id,
      status: PermissionResponseStatus.Rejected,
    });
  };

  return {
    approve,
    canApprove: approvalState.canApprove,
    permissionRequest,
    reject,
    setApprovalState,
  };
});
