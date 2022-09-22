// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Types } from 'aptos';
import { useRef } from 'react';
import { aptosCoinStructTag, coinStoreStructTag } from 'core/constants';
import useCachedRestApi from 'core/hooks/useCachedRestApi';
import { parseMoveAbortDetails } from 'shared/move';
import {
  CoinInfoData,
  CoinStoreResourceData,
  EventHandle,
  Resource,
} from 'shared/types/resource';
import { CoinBalanceChange, Transaction } from 'shared/types/transaction';

// region Utils

/**
 * Check if an event belongs to a specific event handle
 */
function doesEventBelongToHandle(event: Types.Event, eventHandle: EventHandle) {
  return event.guid.account_address === eventHandle.guid.id.addr
    && event.guid.creation_number === eventHandle.guid.id.creation_num;
}

/**
 * Utility function for asynchronously mapping an object using a mapping function
 * @param input input object
 * @param mapFn asynchronous mapping function
 */
async function asyncObjectMap<TInput, TOutput>(
  input: { [key: string]: TInput },
  mapFn: (input: TInput, key: string) => Promise<TOutput>,
) {
  const inputEntries = Object.entries(input);
  const outputEntries = await Promise.all(inputEntries.map(
    async ([key, value]) => [key, await mapFn(value, key)] as const,
  ));
  return Object.fromEntries(outputEntries);
}

// endregion

type ResourcesByAccount = Record<string, Resource[]>;

const coinTransferFunction = '0x1::coin::transfer';
const accountTransferFunction = '0x1::aptos_account::transfer';
const coinMintFunction = '0x1::aptos_coin::mint';

/**
 * Parse the new state of resources affected by a transaction, grouped by owner address
 * @param txn originating transaction
 */
function getNewResourcesStateByAccount(txn: Types.UserTransaction) {
  const newResourcesStateByOwner: ResourcesByAccount = {};
  for (const change of txn.changes) {
    if (change.type === 'write_resource') {
      const { address, data } = change as Types.WriteResource;
      const newResourceState = data as Resource;
      if (address in newResourcesStateByOwner) {
        newResourcesStateByOwner[address].push(newResourceState);
      } else {
        newResourcesStateByOwner[address] = [newResourceState];
      }
    }
  }
  return newResourcesStateByOwner;
}

/**
 * Get coin store resources from a set of resources, grouped by coin type
 */
function getCoinStoresByCoinType(resources: Resource[]) {
  const coinStoreTypePattern = new RegExp(`^${coinStoreStructTag}<(.+)>$`);
  const coinStores: Record<string, CoinStoreResourceData> = {};
  for (const resource of resources) {
    const match = resource.type.match(coinStoreTypePattern);
    if (match !== null) {
      const coinType = match[1];
      coinStores[coinType] = resource.data;
    }
  }
  return coinStores;
}

/**
 * Hook that provides a function for parsing a `Types.Transaction` into
 * a managed transaction
 */
export default function useParseTransaction() {
  const cachedRestApi = useCachedRestApi();

  // A tiny optimization for sharing CoinInfo refs instead of
  // having every `CoinBalanceChange` instance hold a copy
  const coinInfoMap = useRef<Record<string, CoinInfoData>>({}).current;
  async function getCoinInfo(coinType: string) {
    if (coinType in coinInfoMap) {
      return coinInfoMap[coinType];
    }
    const coinInfo = await cachedRestApi.getCoinInfo(coinType);
    if (coinInfo !== undefined) {
      coinInfoMap[coinType] = coinInfo;
    }
    return coinInfo;
  }

  /**
   * Get the coin store balance change from the associated events
   * @param coinType coin type associated to the store
   * @param coinStore coin store resource
   * @param events events emitted during the transaction
   */
  async function getCoinBalanceChange(
    coinType: string,
    coinStore: CoinStoreResourceData,
    events: Types.Event[],
  ) {
    const depositEventHandle = coinStore.deposit_events;
    const withdrawEventHandle = coinStore.withdraw_events;

    let amount = 0n;
    for (const event of events) {
      if (doesEventBelongToHandle(event, depositEventHandle)) {
        amount += BigInt(event.data.amount);
      } else if (doesEventBelongToHandle(event, withdrawEventHandle)) {
        amount -= BigInt(event.data.amount);
      }
    }

    const coinInfo = await getCoinInfo(coinType);
    return {
      amount,
      coinInfo,
    } as CoinBalanceChange;
  }

  return async (txn: Types.UserTransaction): Promise<Transaction> => {
    const resourcesByAccount = getNewResourcesStateByAccount(txn);
    const coinBalanceChangesByAccount = await asyncObjectMap(
      resourcesByAccount,
      async (resources) => {
        const coinStores = getCoinStoresByCoinType(resources);
        const balanceChanges = await asyncObjectMap(
          coinStores,
          async (coinStore, coinType) => getCoinBalanceChange(
            coinType,
            coinStore,
            txn.events,
          ),
        );
        // Filter out zero-sum balance changes
        for (const [coinType, balanceChange] of Object.entries(balanceChanges)) {
          if (balanceChange.amount === 0n) {
            delete balanceChanges[coinType];
          }
        }
        return balanceChanges;
      },
    );

    const timestamp = Math.round(Number(txn.timestamp) / 1000);
    const expirationTimestamp = Number(txn.expiration_timestamp_secs) * 1000;
    const gasFee = Number(txn.gas_used);
    const gasUnitPrice = Number(txn.gas_unit_price);
    const version = Number(txn.version);
    const payload = txn.payload as Types.EntryFunctionPayload;
    const error = !txn.success ? parseMoveAbortDetails(txn.vm_status) : undefined;

    const baseProps = {
      coinBalanceChanges: coinBalanceChangesByAccount,
      error,
      expirationTimestamp,
      gasFee,
      gasUnitPrice,
      hash: txn.hash,
      payload,
      rawChanges: txn.changes,
      success: txn.success,
      timestamp,
      version,
    };

    if (payload.function === coinTransferFunction || payload.function === accountTransferFunction) {
      const recipient = payload.arguments[0];
      const amount = BigInt(payload.arguments[1]);
      const coinType = payload.type_arguments[0] ?? aptosCoinStructTag;
      const coinInfo = await getCoinInfo(coinType);

      return {
        amount,
        coinInfo,
        coinType,
        recipient,
        sender: txn.sender,
        type: 'transfer',
        ...baseProps,
      };
    }

    if (payload.function === coinMintFunction) {
      const recipient = payload.arguments[0];
      const amount = BigInt(payload.arguments[1]);
      const coinInfo = await getCoinInfo(aptosCoinStructTag);
      return {
        amount,
        coinInfo,
        recipient,
        type: 'mint',
        ...baseProps,
      };
    }

    return {
      sender: txn.sender,
      type: 'generic',
      ...baseProps,
    };
  };
}
