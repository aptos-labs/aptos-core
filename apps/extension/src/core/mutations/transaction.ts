// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useToast } from '@chakra-ui/react';
import {
  AptosAccount, AptosClient, MaybeHexString, Types, TxnBuilderTypes, BCS,
} from 'aptos';
import {
  AccountAddress, Identifier, StructTag, TypeTagStruct,
} from 'aptos/dist/transaction_builder/aptos_types';
import useWalletState from 'core/hooks/useWalletState';
import {
  type GetTestCoinTokenBalanceFromAccountResourcesProps,
} from 'core/queries/account';
import queryKeys from 'core/queries/queryKeys';
import { getUserTransaction } from 'core/queries/transaction';
import { useMutation, useQueryClient } from 'react-query';

export interface SubmitTransactionProps {
  fromAccount: AptosAccount;
  nodeUrl: string;
  payload: Types.TransactionPayload,
}

function parseTypeTag(typeTag: string): TxnBuilderTypes.TypeTag {
  if (typeTag.startsWith('vector')) {
    return new TxnBuilderTypes.TypeTagVector(
      parseTypeTag(typeTag.substring(7, typeTag.length - 1)),
    );
  }
  if (typeTag.includes('::')) {
    if (typeTag.split('::').length === 3) {
      return new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString(typeTag));
    }
    if (typeTag.split('::').length === 4) {
      const [address, module, name, tempTypeArgs] = typeTag.split('::');
      const typeArgs = parseTypeTag(tempTypeArgs);
      const structTag = new StructTag(
        AccountAddress.fromHex(address),
        new Identifier(module),
        new Identifier(name),
        [typeArgs],
      );

      return new TypeTagStruct(structTag);
    }
  }

  switch (typeTag) {
    case 'bool':
      return new TxnBuilderTypes.TypeTagBool();
    case 'u8':
      return new TxnBuilderTypes.TypeTagU8();
    case 'u64':
      return new TxnBuilderTypes.TypeTagU64();
    case 'u128':
      return new TxnBuilderTypes.TypeTagU128();
    case 'address':
      return new TxnBuilderTypes.TypeTagAddress();
    case 'signer':
      return new TxnBuilderTypes.TypeTagSigner();
    default:
      throw new Error('Unknown type tag');
  }
}

export const submitTransaction = async ({
  fromAccount,
  nodeUrl,
  payload,
}: SubmitTransactionProps) => {
  const client = new AptosClient(nodeUrl);
  const txnRequest = await client.generateTransaction(fromAccount.address(), payload);

  if (payload.type === 'script_function_payload') {
    if ('function' in payload) {
      const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
        client.getAccount(fromAccount.address()),
        client.getChainId(),
      ]);

      const payloadFunctionId = payload.function.split('::');

      const tokens: TxnBuilderTypes.TypeTag[] = [];
      payload.type_arguments.forEach((value) => {
        tokens.push(parseTypeTag(value));
      });

      const bcsPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
        TxnBuilderTypes.ScriptFunction.natural(
          `${payloadFunctionId[0]}::${payloadFunctionId[1]}`,
          payloadFunctionId[2],
          tokens,
          [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(payload.arguments[0])),
            BCS.bcsSerializeUint64(payload.arguments[1])],
        ),
      );

      const rawTxn = new TxnBuilderTypes.RawTransaction(
        TxnBuilderTypes.AccountAddress.fromHex(fromAccount.address()),
        BigInt(sequnceNumber),
        bcsPayload,
        BigInt(1000),
        BigInt(1),
        BigInt(Math.floor(Date.now() / 1000) + 10),
        new TxnBuilderTypes.ChainId(chainId),
      );

      const signedTxn = AptosClient.generateBCSTransaction(fromAccount, rawTxn);
      const transactionRes = await client.submitSignedBCSTransaction(signedTxn);
      await client.waitForTransaction(transactionRes.hash);
      return transactionRes.hash;
    }
  }

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
  nodeUrl,
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

export const useSubmitTestCoinTransfer = () => {
  const { aptosNetwork } = useWalletState();
  const queryClient = useQueryClient();
  const toast = useToast();

  return useMutation(submitTestCoinTransferTransaction, {
    onSettled: async (txnHash) => {
      if (!txnHash) {
        return;
      }
      queryClient.invalidateQueries(queryKeys.getAccountResources);
      const txn = await getUserTransaction({ nodeUrl: aptosNetwork, txnHashOrVersion: txnHash });
      const amount = (txn?.payload)
        ? (txn.payload as { arguments: string[] }).arguments[1]
        : undefined;
      toast({
        description: (txn?.success) ? `Amount transferred: ${amount}, gas consumed: ${txn?.gas_used}` : `Transfer failed, gas consumed: ${txn?.gas_used}`,
        duration: 5000,
        isClosable: true,
        status: (txn?.success) ? 'success' : 'error',
        title: `Transaction ${txn?.success ? 'success' : 'error'}`,
        variant: 'solid',
      });
    },
  });
};
