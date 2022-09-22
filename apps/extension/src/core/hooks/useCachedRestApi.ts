// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Types } from 'aptos';
import { openDB, DBSchema } from 'idb';
import { useRef } from 'react';
import { useNetworks } from 'core/hooks/useNetworks';
import { useFetchAccountResource } from 'core/queries/useAccountResources';
import { EventWithVersion, Event } from 'shared/types/event';
import { CoinInfoData, CoinInfoResource } from 'shared/types/resource';

function parseRawEvent(event: EventWithVersion) {
  return {
    data: event.data,
    guid: {
      address: event.guid.account_address,
      creationNumber: Number(event.guid.creation_number),
    },
    sequenceNumber: Number(event.sequence_number),
    type: event.type,
    version: Number(event.version),
  } as Event;
}

interface RestCacheDbSchema extends DBSchema {
  coins: {
    key: string,
    value: CoinInfoData,
  },
  events: {
    indexes: { byEventKey: [string, number, number] },
    key: string,
    value: Event
  },
  transactions: {
    key: number,
    value: Types.UserTransaction,
  },
}

interface RestCacheDbMeta {
  chainId: number,
  nodeUrl: string
}

interface MetaDbSchema extends DBSchema {
  restCacheDbs: {
    key: string,
    value: RestCacheDbMeta
  },
}

export default function useCachedRestApi() {
  const { activeNetwork, aptosClient } = useNetworks();
  const fetchAccountResource = useFetchAccountResource();

  const restCacheDbMeta = useRef<RestCacheDbMeta>();

  /**
   * Opens a connection to the meta IndexedDB
   */
  const getMetaDb = async () => openDB<MetaDbSchema>('meta', 1, {
    upgrade: (conn) => {
      conn.createObjectStore('restCacheDbs');
    },
  });

  /**
   * Opens a connection to the cache IndexedDB
   */
  const getConnection = async () => {
    const chainId = await aptosClient.getChainId();
    const chainKey = `${activeNetwork.nodeUrl}_${chainId}`;
    const dbName = `restCache_${chainKey}`;

    const connection = openDB<RestCacheDbSchema>(dbName, 1, {
      upgrade: (conn) => {
        conn.createObjectStore('transactions');
        const eventsStore = conn.createObjectStore('events');
        eventsStore.createIndex('byEventKey', [
          'guid.address',
          'guid.creationNumber',
          'sequenceNumber',
        ]);
        conn.createObjectStore('coins');
      },
    });

    // Initialize dbMeta reference if needed
    if (!restCacheDbMeta.current) {
      const metaDb = await getMetaDb();
      restCacheDbMeta.current = await metaDb.get('restCacheDbs', dbName);
      if (!restCacheDbMeta.current) {
        restCacheDbMeta.current = {
          chainId,
          nodeUrl: activeNetwork.nodeUrl,
        };
        await metaDb.put('restCacheDbs', restCacheDbMeta.current, dbName);
      }
    }

    return connection;
  };

  /**
   * Get info for the specified coin type.
   * If not available in cache, the value is fetched using the active AptosClient
   * and added to the cache.
   * @param coinType
   */
  const getCoinInfo = async (coinType: string) => {
    const coinAddress = coinType.split('::')[0];
    const conn = await getConnection();

    const cachedCoinInfo = await conn.get('coins', coinType) as CoinInfoData;
    if (cachedCoinInfo !== undefined) {
      return cachedCoinInfo;
    }

    const coinInfoResourceType = `0x1::coin::CoinInfo<${coinType}>`;
    const coinInfoResource = await fetchAccountResource(
      coinAddress,
      coinInfoResourceType,
    ) as CoinInfoResource | undefined;
    if (coinInfoResource === undefined) {
      return undefined;
    }

    const coinInfo = coinInfoResource.data;
    delete coinInfo.supply;
    await conn.put('coins', coinInfo, coinType);
    return coinInfo as CoinInfoData;
  };

  /**
   * Get transaction by version.
   * If not available in cache, the value is fetched using the active AptosClient
   * and added to the cache.
   * @param version
   */
  const getTransaction = async (version: number) => {
    const conn = await getConnection();
    const cachedTxn = await conn.get('transactions', version);
    if (cachedTxn !== undefined) {
      return cachedTxn as Types.UserTransaction;
    }

    const txn = await aptosClient.getTransactionByVersion(version) as Types.UserTransaction;
    await conn.put('transactions', txn, version);
    return txn;
  };

  /**
   * Get events for a specific event key and range.
   * The event key is defined by [address, creationNumber].
   * @param address address of resource owner account
   * @param creationNumber creation number of the event table
   * @param start creation number from where to start querying events
   * @param limit number of events to query. Needs to be strictly greater than 0
   */
  const getEvents = async (
    address: string,
    creationNumber: number,
    start: number,
    limit: number,
  ) => {
    const conn = await getConnection();

    // For now this is using cached values only for perfect matches
    // can be optimized
    const query = IDBKeyRange.bound(
      [address, creationNumber, start],
      [address, creationNumber, start + limit],
      false,
      true,
    );

    const cachedEvents = await conn.getAllFromIndex('events', 'byEventKey', query);
    if (cachedEvents.length === limit) {
      return cachedEvents as Event[];
    }

    const newRawEvents = (await aptosClient.getEventsByCreationNumber(
      address,
      creationNumber,
      { limit, start },
    )) as EventWithVersion[];
    const newEvents = newRawEvents.map((e) => parseRawEvent(e));

    const dbTxn = conn.transaction('events', 'readwrite');
    await Promise.all(newEvents.map(async (event) => {
      const key = `${address}_${creationNumber}_${event.sequenceNumber}`;
      await dbTxn.store.put(event, key);
    }));
    await dbTxn.done;

    return newEvents;
  };

  return {
    getCoinInfo,
    getEvents,
    getTransaction,
  };
}
