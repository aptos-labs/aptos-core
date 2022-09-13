// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PublicAccount } from 'shared/types';

export interface ConnectPermissionApproval {
  account: PublicAccount,
}

export interface SignAndSubmitTransactionPermissionApproval {
  maxGasFee: number,
}

export interface SignTransactionPermissionApproval {
  maxGasFee: number,
}

export type PermissionApproval = ConnectPermissionApproval
| SignAndSubmitTransactionPermissionApproval
| SignTransactionPermissionApproval
| undefined;
