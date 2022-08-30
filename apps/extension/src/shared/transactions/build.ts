// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  BCS, MaybeHexString, TransactionBuilder, TxnBuilderTypes,
} from 'aptos';
import {
  AccountAddress,
  ChainId,
  EntryFunction,
  RawTransaction,
  StructTag,
  TransactionPayload,
  TransactionPayloadEntryFunction,
  TypeTagStruct,
} from 'aptos/dist/transaction_builder/aptos_types';
import { accountNamespace, aptosCoinStructTag, coinNamespace } from 'core/constants';

export interface TransactionOptions {
  expirationSecondsFromNow?: number,
  gasUnitPrice?: number | bigint,
  maxGasAmount?: number | bigint,
}

export const defaultTransactionOptions = {
  expirationSecondsFromNow: 20,
  gasUnitPrice: 1n,
  maxGasAmount: 10000n,
};

export function buildRawTransaction(
  senderAddress: MaybeHexString,
  sequenceNumber: number | bigint,
  chainId: number,
  payload: TransactionPayload,
  options?: TransactionOptions,
): TxnBuilderTypes.RawTransaction {
  const {
    expirationSecondsFromNow,
    gasUnitPrice,
    maxGasAmount,
  } = { ...defaultTransactionOptions, ...options };

  const expirationTimestamp = Math.floor(Date.now() / 1000) + expirationSecondsFromNow;

  return new RawTransaction(
    AccountAddress.fromHex(senderAddress),
    BigInt(sequenceNumber),
    payload,
    BigInt(maxGasAmount),
    BigInt(gasUnitPrice),
    BigInt(expirationTimestamp),
    new ChainId(Number(chainId)),
  );
}

export function getSigningMessage(rawTransaction: TxnBuilderTypes.RawTransaction) {
  return TransactionBuilder.getSigningMessage(rawTransaction).toString('hex');
}

/**
 * Create an account creation transaction payload
 * @param address address for which to create an account
 */
export function buildCreateAccountPayload(address: MaybeHexString) {
  const encodedArgs = [
    BCS.bcsToBytes(AccountAddress.fromHex(address)),
  ];

  const entryFunction = EntryFunction.natural(accountNamespace, 'create_account', [], encodedArgs);
  return new TransactionPayloadEntryFunction(entryFunction);
}

/**
 * Create a coin transfer transaction payload
 * @param recipient recipient address
 * @param amount amount of coins to transfer
 */
export function buildCoinTransferPayload(recipient: MaybeHexString, amount: number) {
  const typeArgs = [
    new TypeTagStruct(StructTag.fromString(aptosCoinStructTag)),
  ];

  const encodedArgs = [
    BCS.bcsToBytes(AccountAddress.fromHex(recipient)),
    BCS.bcsSerializeUint64(BigInt(amount)),
  ];

  const entryFunction = EntryFunction.natural(coinNamespace, 'transfer', typeArgs, encodedArgs);
  return new TransactionPayloadEntryFunction(entryFunction);
}

/**
 * Create an account coin transfer transaction payload.
 * This differs from 0x1::coin::transfer in that
 * it creates the recipient account if it doesn't exist
 * @param recipient recipient address
 * @param amount amount of coins to transfer
 */
export function buildAccountTransferPayload(recipient: MaybeHexString, amount: number) {
  const encodedArgs = [
    BCS.bcsToBytes(AccountAddress.fromHex(recipient)),
    BCS.bcsSerializeUint64(BigInt(amount)),
  ];

  const entryFunction = EntryFunction.natural(accountNamespace, 'transfer', [], encodedArgs);
  return new TransactionPayloadEntryFunction(entryFunction);
}
