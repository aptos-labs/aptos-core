// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// See https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move
export enum CoinTransferErrorReason {
  CoinInfoAddressMismatch = 0,
  CoinInfoAlreadyPublished,
  CoinInfoNotPublished,
  CoinStoreAlreadyPublished,
  CoinStoreNotPublished,
  InsufficientBalance,
  DestructionOfNonZeroToken,
  TotalSupplyOverflow,
  InvalidCoinAmount,
}

export default CoinTransferErrorReason;
