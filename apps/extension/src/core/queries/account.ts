// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosClient, MaybeHexString,
} from 'aptos';
import useWalletState from 'core/hooks/useWalletState';
import { useQuery, useQueryClient } from 'react-query';
import { aptosCoinStoreStructTag } from 'core/constants';

export interface GetAccountResourcesProps {
  address?: MaybeHexString;
  nodeUrl: string;
}

export const getAccountResources = async ({
  address,
  nodeUrl,
}: GetAccountResourcesProps) => {
  const client = new AptosClient(nodeUrl);
  return (address) ? (client.getAccountResources(address)) : undefined;
};

export const getAccountExists = async ({
  address,
  nodeUrl,
}: GetAccountResourcesProps) => {
  const client = new AptosClient(nodeUrl);
  try {
    const account = await client.getAccount(address!);
    return !!(account);
  } catch (err) {
    return false;
  }
};

export const accountQueryKeys = Object.freeze({
  getAccountCoinBalance: 'getAccountCoinBalance',
  getAccountExists: 'getAccountExists',
  getSequenceNumber: 'getSequenceNumber',
} as const);

interface UseAccountExistsProps {
  address?: MaybeHexString;
}

/**
 * Check whether an account associated to the specified address exists
 */
export const useAccountExists = ({
  address,
}: UseAccountExistsProps) => {
  const { nodeUrl } = useWalletState();

  return useQuery(
    [accountQueryKeys.getAccountExists, address],
    async () => getAccountExists({ address: address!, nodeUrl }),
    { enabled: Boolean(address) },
  );
};

interface UseAccountCoinBalanceParams {
  address?: string,
  refetchInterval?: number | false,
}

/**
 * Query coin balance for the current account
 * @param refetchInterval automatic refetch interval in milliseconds
 */
export function useAccountCoinBalance({
  address,
  refetchInterval,
}: UseAccountCoinBalanceParams = {}) {
  const { aptosAccount, nodeUrl } = useWalletState();

  const accountAddress = address || aptosAccount?.address()?.hex();

  return useQuery([accountQueryKeys.getAccountCoinBalance, accountAddress], async () => {
    const client = new AptosClient(nodeUrl);
    const resource: any = await client.getAccountResource(accountAddress!, aptosCoinStoreStructTag);
    return Number(resource.data.coin.value);
  }, {
    enabled: Boolean(accountAddress),
    refetchInterval,
  });
}

/**
 * Query sequence number for current account,
 * which is required to BCD-encode a transaction locally.
 * The value is queried lazily the first time `get` is called, and is
 * refetched only when an error occurs, by invalidating the cache or
 * manually refetching.
 */
export function useSequenceNumber() {
  const { aptosAccount, nodeUrl } = useWalletState();
  const accountAddress = aptosAccount?.address()?.hex();
  const queryClient = useQueryClient();

  const queryKey = [accountQueryKeys.getSequenceNumber, nodeUrl, accountAddress];

  const { refetch } = useQuery(queryKey, async () => {
    if (!accountAddress) {
      return undefined;
    }
    const aptosClient = new AptosClient(nodeUrl);
    const account = await aptosClient.getAccount(accountAddress!);
    return Number(account.sequence_number);
  }, { enabled: false });

  return {
    get: async () => {
      const value = queryClient.getQueryData<number>(queryKey);
      return value !== undefined
        ? value
        : (await refetch({ throwOnError: true })).data!;
    },
    increment: () => queryClient.setQueryData<number | undefined>(
      queryKey,
      (prev?: number) => prev && prev + 1,
    ),
    refetch,
  };
}
