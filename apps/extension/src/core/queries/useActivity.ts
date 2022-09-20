// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Types } from 'aptos';
import {
  UseInfiniteQueryOptions,
  useInfiniteQuery,
} from 'react-query';

import { aptosCoinStoreStructTag } from 'core/constants';
import { useActiveAccount } from 'core/hooks/useAccounts';
import useCachedRestApi from 'core/hooks/useCachedRestApi';
import { Event } from 'core/types/event';
import { EventHandle } from 'core/types/resource';
import { useFetchAccountResources } from './useAccountResources';

export const getActivityQueryKey = 'getActivity';
export const defaultEventQueryStep = 10;
export const defaultPageSize = 20;

const aptosCoinStoreDepositEventsKey = {
  eventName: 'deposit_events',
  resourceType: aptosCoinStoreStructTag,
};

const aptosCoinStoreWithdrawEventsKey = {
  eventName: 'withdraw_events',
  resourceType: aptosCoinStoreStructTag,
};

const managedEventsKeys = [
  aptosCoinStoreDepositEventsKey,
  aptosCoinStoreWithdrawEventsKey,
] as const;

interface EventBuffer {
  cursor: number,
  data: Event[],
}

interface ActivityBuffers {
  events: { [creationNum: number]: EventBuffer },
  txns: Types.UserTransaction[],
}

interface ActivityQueryPage {
  buffers: ActivityBuffers,
  txns: Types.UserTransaction[],
}

export interface UseActivityProps {
  eventQueryStep?: number,
  pageSize?: number,
}

export default function useActivity(
  options?: UseInfiniteQueryOptions<ActivityQueryPage> & UseActivityProps,
) {
  const {
    eventQueryStep,
    pageSize,
    ...queryOptions
  } = {
    cacheTime: 0,
    eventQueryStep: defaultEventQueryStep,
    pageSize: defaultPageSize,
    retry: false,
    staleTime: Infinity,
    ...options,
  };

  const { activeAccountAddress: address } = useActiveAccount();
  const { getEvents, getTransaction } = useCachedRestApi();
  const fetchResources = useFetchAccountResources(address);

  const loadMore = async (prevBuffers: ActivityBuffers) => {
    let newStartVersion = 0;
    const currBuffers = {
      events: { ...prevBuffers.events },
      txns: [...prevBuffers.txns],
    };

    // Load more events into current buffers
    await Promise.all(Object.entries(currBuffers.events).map(async ([
      creationNum,
      { cursor, data },
    ]) => {
      const start = Math.max(cursor - eventQueryStep, 0);
      const limit = cursor - data.length - start;

      if (limit > 0) {
        const newEvents = await getEvents(
          address,
          Number(creationNum),
          start,
          limit,
        );
        data.push(...newEvents.reverse());
      }

      if (data.length > 0) {
        const currEventStartVersion = data[data.length - 1].version;
        newStartVersion = Math.max(newStartVersion, currEventStartVersion);
      }
    }));

    const mergedVersions = new Set<number>();
    const shouldExtract = (e: Event) => e.version < newStartVersion;

    for (const eventBuffer of Object.values(currBuffers.events)) {
      const firstIdxToExtract = eventBuffer.data.findIndex((e) => shouldExtract(e));
      const nToExtract = firstIdxToExtract >= 0 ? firstIdxToExtract : eventBuffer.data.length;
      if (nToExtract > 0) {
        eventBuffer.cursor = Math.max(eventBuffer.cursor - nToExtract, 0);
        eventBuffer.data.splice(0, nToExtract)
          .forEach(({ version }) => mergedVersions.add(version));
      }
    }

    // Fetch relative transactions, sorted by descending version
    const newTransactions = await Promise.all(
      Array.from(mergedVersions)
        .sort((a, b) => b - a)
        .map((version) => getTransaction(version)),
    );

    currBuffers.txns.push(...newTransactions);
    return currBuffers;
  };

  const loadActivityPage = async (prevBuffers: ActivityBuffers) => {
    let currBuffers = prevBuffers;
    const pageTxns = currBuffers.txns.splice(0, pageSize);
    while (pageTxns.length < pageSize) {
      const hasMoreEvents = Object.values(currBuffers.events).some(({ cursor }) => cursor > 0);
      if (!hasMoreEvents) {
        break;
      }
      // eslint-disable-next-line no-await-in-loop
      currBuffers = await loadMore(currBuffers);
      const nRemaining = pageSize - pageTxns.length;
      const extracted = currBuffers.txns.splice(0, nRemaining);
      pageTxns.push(...extracted);
    }
    return { buffers: currBuffers, txns: pageTxns };
  };

  const initializeBuffers = async () => {
    const initialBuffers: ActivityBuffers = { events: {}, txns: [] };
    const resources = await fetchResources();

    managedEventsKeys.forEach(({ eventName, resourceType }) => {
      const resource = resources.find((res) => res.type === resourceType)?.data as any;
      if (resource && resource[eventName]) {
        const resourceEvent = resource[eventName] as EventHandle;
        const count = Number(resourceEvent.counter);
        const creationNum = Number(resourceEvent.guid.id.creation_num);
        initialBuffers.events[creationNum] = { cursor: count, data: [] };
      }
    });
    return initialBuffers;
  };

  return useInfiniteQuery<ActivityQueryPage>(
    [getActivityQueryKey, address],
    async ({ pageParam: prevBuffers }) => loadActivityPage(
      prevBuffers ?? await initializeBuffers(),
    ),
    {
      getNextPageParam: ({ buffers }: ActivityQueryPage) => {
        const hasMore = buffers.txns.length > 0
          || Object.values(buffers.events).some(({ cursor }) => cursor > 0);
        return hasMore ? buffers : undefined;
      },
      ...queryOptions,
    },
  );
}
