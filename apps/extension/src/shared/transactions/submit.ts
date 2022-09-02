// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosAccount,
  AptosClient,
  Types,
  TxnBuilderTypes,
} from 'aptos';

import { handleApiError, throwForVmError } from './utils';

export async function simulateTransaction(
  aptosAccount: AptosAccount,
  aptosClient: AptosClient,
  rawTxn: TxnBuilderTypes.RawTransaction,
) {
  const simulatedTxn = AptosClient.generateBCSSimulation(aptosAccount, rawTxn);
  try {
    const userTxn = (await aptosClient.submitBCSSimulation(simulatedTxn))[0];
    throwForVmError(userTxn);
    return userTxn;
  } catch (err) {
    handleApiError(err);
    throw err;
  }
}

export async function submitTransaction(
  aptosAccount: AptosAccount,
  aptosClient: AptosClient,
  rawTxn: TxnBuilderTypes.RawTransaction,
) {
  const signedTxn = AptosClient.generateBCSTransaction(aptosAccount, rawTxn);
  try {
    const { hash } = await aptosClient.submitSignedBCSTransaction(signedTxn);
    return await aptosClient.waitForTransactionWithResult(hash) as Types.UserTransaction;
  } catch (err) {
    handleApiError(err);
    throw err;
  }
}
