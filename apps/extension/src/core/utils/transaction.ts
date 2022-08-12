// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  BCS,
  MaybeHexString,
  TransactionBuilder,
  TxnBuilderTypes,
} from 'aptos';
import { accountNamespace, aptosCoinStructTag, coinNamespace } from 'core/constants';

const {
  AccountAddress,
  ChainId,
  RawTransaction,
  ScriptFunction,
  StructTag,
  TransactionPayloadScriptFunction,
  TypeTagStruct,
} = TxnBuilderTypes;

export interface TransactionConfig {
  chainId: number,
  expSecFromNow?: number,
  gasAmount?: number | bigint,
  gasUnitPrice?: number | bigint,
  sender: MaybeHexString,
  sequenceNumber: number | bigint,
}

export function createRawTransaction(
  transactionPayload: TxnBuilderTypes.TransactionPayload,
  config: TransactionConfig,
): TxnBuilderTypes.RawTransaction {
  const {
    chainId, expSecFromNow, gasAmount, gasUnitPrice, sender, sequenceNumber,
  } = {
    expSecFromNow: Math.floor(Date.now() / 1000) + 10,
    gasAmount: 1_000_000,
    gasUnitPrice: 1,
    ...config,
  };

  return new RawTransaction(
    AccountAddress.fromHex(sender),
    BigInt(sequenceNumber),
    transactionPayload,
    BigInt(gasAmount), // TODO: wallet needs to ask users for how much gas to use here
    BigInt(gasUnitPrice), // Unit gas price
    BigInt(expSecFromNow), // Transactions are valid for 20s before expires
    new ChainId(Number(chainId)),
  );
}

export function getSigningMessage(rawTransaction: TxnBuilderTypes.RawTransaction) {
  return TransactionBuilder.getSigningMessage(rawTransaction).toString('hex');
}

/**
 * Create a coin transfer transaction payload
 * @param recipient recipient address
 * @param amount amount of coins to transfer
 */
export function buildCoinTransferPayload(recipient: MaybeHexString, amount: bigint) {
  const typeArgs = [
    new TypeTagStruct(StructTag.fromString(aptosCoinStructTag)),
  ];

  const encodedArgs = [
    BCS.bcsToBytes(AccountAddress.fromHex(recipient)),
    BCS.bcsSerializeUint64(BigInt(amount)),
  ];

  const scriptFunction = ScriptFunction.natural(coinNamespace, 'transfer', typeArgs, encodedArgs);
  return new TransactionPayloadScriptFunction(scriptFunction);
}

/**
 * Create an account creation transaction payload
 * @param address address for which to create an account
 */
export function buildCreateAccountPayload(address: MaybeHexString) {
  const encodedArgs = [
    BCS.bcsToBytes(AccountAddress.fromHex(address)),
  ];

  const scriptFunction = ScriptFunction.natural(accountNamespace, 'create_account', [], encodedArgs);
  return new TransactionPayloadScriptFunction(scriptFunction);
}

/**
 * Create an account coin transfer transaction payload.
 * This differs from 0x1::coin::transfer in that
 * it creates the recipient account if it doesn't exist
 * @param recipient recipient address
 * @param amount amount of coins to transfer
 */
export function buildAccountTransferPayload(recipient: MaybeHexString, amount: bigint) {
  const encodedArgs = [
    BCS.bcsToBytes(AccountAddress.fromHex(recipient)),
    BCS.bcsSerializeUint64(BigInt(amount)),
  ];

  const scriptFunction = ScriptFunction.natural(accountNamespace, 'transfer', [], encodedArgs);
  return new TransactionPayloadScriptFunction(scriptFunction);
}
