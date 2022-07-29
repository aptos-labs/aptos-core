// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosClient,
  MaybeHexString,
  TxnBuilderTypes,
  BCS,
  RequestError,
} from 'aptos';
import { toast } from 'core/components/Toast';
import useWalletState from 'core/hooks/useWalletState';
import { useSequenceNumber } from 'core/queries/account';
import queryKeys from 'core/queries/queryKeys';
import Analytics from 'core/utils/analytics/analytics';
import { coinEvents } from 'core/utils/analytics/events';
import { useMutation, useQuery, useQueryClient } from 'react-query';
import { AptosError, ScriptFunctionPayload, UserTransaction } from 'aptos/dist/api/data-contracts';
import { useChainId } from 'core/queries/network';
import { aptosCoinStructTag, coinNamespace } from 'core/constants';
import { MoveExecutionStatus, parseMoveVmStatus } from 'core/utils/move';

/* function parseTypeTag(typeTag: string): TxnBuilderTypes.TypeTag {
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
} */

interface CoinTransferRequestParams {
  amount: number,
  chainId: number,
  recipient: MaybeHexString,
  sender: MaybeHexString,
  sequenceNumber: number
}

/**
 * Create a coin transfer BCS-encoded transaction
 * @param amount amount of coins to transfer
 * @param chainId (required for encoding locally)
 * @param recipient recipient address
 * @param sender sender address
 * @param sequenceNumber (required for encoding locally)
 */
function createCoinTransferTransaction({
  amount,
  chainId,
  recipient,
  sender,
  sequenceNumber,
}: CoinTransferRequestParams) {
  const {
    AccountAddress,
    ChainId,
    RawTransaction,
    ScriptFunction,
    StructTag,
    TransactionPayloadScriptFunction,
    TypeTagStruct,
  } = TxnBuilderTypes;

  const typeArgs = [
    new TypeTagStruct(StructTag.fromString(aptosCoinStructTag)),
  ];

  const encodedArgs = [
    BCS.bcsToBytes(AccountAddress.fromHex(recipient)),
    BCS.bcsSerializeUint64(BigInt(amount)),
  ];

  const scriptFunction = ScriptFunction.natural(coinNamespace, 'transfer', typeArgs, encodedArgs);
  const encodedPayload = new TransactionPayloadScriptFunction(scriptFunction);

  return new RawTransaction(
    AccountAddress.fromHex(sender),
    BigInt(sequenceNumber),
    encodedPayload,
    BigInt(1000),
    BigInt(1),
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new ChainId(chainId),
  );
}

/**
 * Get a raw coin transfer transaction factory for the current account
 */
function useCreateCoinTransferTransaction() {
  const { aptosAccount } = useWalletState();
  const { data: chainId } = useChainId();
  const { get: getSequenceNumber } = useSequenceNumber();

  const sender = aptosAccount?.address().hex();
  const isReady = sender && chainId !== undefined;

  return isReady
    ? async ({
      amount,
      recipient,
    }: SubmitCoinTransferParams) => createCoinTransferTransaction({
      amount,
      chainId,
      recipient,
      sender,
      sequenceNumber: await getSequenceNumber(),
    })
    : undefined;
}

export interface UseCoinTransferParams {
  amount?: number,
  enabled?: boolean,
  recipient?: string,
}

/**
 * Query a coin transfer simulation for the specified recipient and amount
 */
export function useCoinTransferSimulation({
  amount,
  enabled,
  recipient,
} : UseCoinTransferParams) {
  const { aptosAccount, nodeUrl } = useWalletState();
  const { refetch: refetchSeqNumber } = useSequenceNumber();
  const createTxn = useCreateCoinTransferTransaction();

  const isReady = Boolean(aptosAccount && createTxn);
  const isInputValid = Boolean(amount && recipient);

  return useQuery(
    [queryKeys.getCoinTransferSimulation, recipient, amount],
    async () => {
      const rawTxn = await createTxn!({
        amount: amount!,
        recipient: recipient!,
      });

      const aptosClient = new AptosClient(nodeUrl);
      const simulatedTxn = AptosClient.generateBCSSimulation(aptosAccount!, rawTxn);
      const userTxn = (await aptosClient.submitBCSSimulation(simulatedTxn)) as UserTransaction;
      if (!userTxn.success) {
        // Miscellaneous error is probably associated with invalid sequence number
        if (parseMoveVmStatus(userTxn.vm_status) === MoveExecutionStatus.MiscellaneousError) {
          await refetchSeqNumber();
          throw new Error(userTxn.vm_status);
        }
      }
      return userTxn;
    },
    {
      cacheTime: 0,
      enabled: isReady && enabled && isInputValid,
      keepPreviousData: true,
      refetchInterval: 5000,
      retry: 1,
    },
  );
}

export interface SubmitCoinTransferParams {
  amount: number,
  recipient: MaybeHexString,
}

/**
 * Mutation for submitting a coin transfer transaction
 */
export function useCoinTransferTransaction() {
  const { aptosAccount, nodeUrl } = useWalletState();
  const {
    increment: incrementSeqNumber,
    refetch: refetchSeqNumber,
  } = useSequenceNumber();
  const createTxn = useCreateCoinTransferTransaction();
  const queryClient = useQueryClient();

  const isReady = Boolean(aptosAccount && createTxn);

  const submitCoinTransferTransaction = async ({
    amount,
    recipient,
  }: SubmitCoinTransferParams) => {
    const rawTxn = await createTxn!({ amount, recipient });

    const aptosClient = new AptosClient(nodeUrl);
    const signedTxn = AptosClient.generateBCSTransaction(aptosAccount!, rawTxn);

    try {
      const { hash } = await aptosClient.submitSignedBCSTransaction(signedTxn);
      await aptosClient.waitForTransaction(hash);
      return (await aptosClient.getTransaction(hash)) as UserTransaction;
    } catch (err) {
      if (err instanceof RequestError) {
        const errorMsg = (err.response?.data as AptosError)?.message;
        if (errorMsg && errorMsg.indexOf('SEQUENCE_NUMBER_TOO_OLD') >= 0) {
          await refetchSeqNumber();
        }
      }
      throw err;
    }
  };

  const mutation = useMutation(submitCoinTransferTransaction, {
    onSuccess: async (txn: UserTransaction, { amount }: SubmitCoinTransferParams) => {
      // Optimistic update of sequence number
      incrementSeqNumber();
      queryClient.invalidateQueries(queryKeys.getAccountCoinBalance);

      const eventType = txn.success
        ? coinEvents.TRANSFER_APTOS_COIN
        : coinEvents.ERROR_TRANSFER_APTOS_COIN;

      const payload = txn.payload as ScriptFunctionPayload;
      const coinType = payload.type_arguments[0];

      const params = {
        amount,
        coinType,
        fromAddress: txn.sender,
        network: nodeUrl,
        ...txn,
      };

      Analytics.event({ eventType, params });

      toast({
        description: (txn.success)
          ? `Amount transferred: ${amount}, gas consumed: ${txn.gas_used}`
          : `Transfer failed, gas consumed: ${txn.gas_used}`,
        status: txn.success ? 'success' : 'error',
        title: `Transaction ${txn.success ? 'success' : 'error'}`,
      });
    },
    retry: 1,
  });

  return { isReady, ...mutation };
}

export const TransferResult = Object.freeze({
  AmountOverLimit: 'Amount is over limit',
  AmountWithGasOverLimit: 'Amount with gas is over limit',
  IncorrectPayload: 'Incorrect transaction payload',
  Success: 'Transaction executed successfully',
  UndefinedAccount: 'Account does not exist',
} as const);
