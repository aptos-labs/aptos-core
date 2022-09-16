// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PublicAccount } from 'shared/types';

export interface ConnectPermissionApproval {
  account: PublicAccount,
}

export interface SignTransactionPermissionApproval {
  gasUnitPrice: number,
  maxGasFee: number,
}

export type PermissionApproval = ConnectPermissionApproval
| SignTransactionPermissionApproval
| undefined;
