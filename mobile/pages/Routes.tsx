// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export interface WalletParams {
  address: string;
}

export type StackParamList = {
  Login: {};
  Wallet: WalletParams;
};
