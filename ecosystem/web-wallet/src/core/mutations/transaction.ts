// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosAccount, AptosClient, MaybeHexString, Types,
} from 'aptos';
import { NODE_URL } from 'core/constants';
import {
  type GetTestCoinTokenBalanceFromAccountResourcesProps,
} from 'core/queries/account';

export interface SubmitTransactionProps {
  fromAccount: AptosAccount;
  nodeUrl?: string;
  payload: Types.TransactionPayload,
}

export const submitTransaction = async ({
  fromAccount,
  nodeUrl = NODE_URL,
  payload,
}: SubmitTransactionProps) => {
  const client = new AptosClient(nodeUrl);
  const txnRequest = await client.generateTransaction(fromAccount.address(), payload);
  const signedTxn = await client.signTransaction(fromAccount, txnRequest);
  const transactionRes = await client.submitTransaction(signedTxn);
  await client.waitForTransaction(transactionRes.hash);
  return transactionRes.hash;
};

export interface TestCoinTransferTransactionPayload {
  amount: string | number;
  toAddress: MaybeHexString;
}

export type SendTestCoinTransactionProps = Omit<SubmitTransactionProps & TestCoinTransferTransactionPayload, 'payload'>;

export const sendTestCoinTransaction = async ({
  amount,
  fromAccount,
  nodeUrl = NODE_URL,
  toAddress,
}: SendTestCoinTransactionProps) => {
  const payload: Types.TransactionPayload = {
    arguments: [toAddress, `${amount}`],
    function: '0x1::Coin::transfer',
    type: 'script_function_payload',
    type_arguments: ['0x1::TestCoin::TestCoin'],
  };
  const txnHash = await submitTransaction({
    fromAccount,
    nodeUrl,
    payload,
  });
  return txnHash;
};

export const TransferResult = Object.freeze({
  AmountOverLimit: 'Amount is over limit',
  AmountWithGasOverLimit: 'Amount with gas is over limit',
  IncorrectPayload: 'Incorrect transaction payload',
  Success: 'Transaction executed successfully',
  UndefinedAccount: 'Account does not exist',
} as const);

export type SubmitTestCoinTransferTransactionProps = Omit<
TestCoinTransferTransactionPayload &
SendTestCoinTransactionProps &
GetTestCoinTokenBalanceFromAccountResourcesProps & {
  onClose: () => void
},
'accountResources'
>;

export const submitTestCoinTransferTransaction = async ({
  amount,
  fromAccount,
  nodeUrl,
  onClose,
  toAddress,
}: SubmitTestCoinTransferTransactionProps) => {
  const txnHash = await sendTestCoinTransaction({
    amount,
    fromAccount,
    nodeUrl,
    toAddress,
  });
  onClose();
  return txnHash;
};
