// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, MaybeHexString } from 'aptos';
import { useQuery, useQueryClient, UseQueryOptions } from 'react-query';
import { aptosCoinStoreStructTag, aptosStakePoolStructTag } from 'core/constants';
import { useNetworks } from 'core/hooks/useNetworks';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { ApiError } from 'aptos/dist/generated';

/**
 * QUERY KEYS
 */
export const accountQueryKeys = Object.freeze({
  getAccountCoinBalance: 'getAccountCoinBalance',
  getAccountExists: 'getAccountExists',
  getAccountStakeBalance: 'getAccountStakeBalance',
  getAccountStakeInfo: 'getAccountStakeInfo',
  getSequenceNumber: 'getSequenceNumber',
} as const);

// ------------------------------------------------------------------------- //
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

interface UseAccountExistsProps {
  address?: MaybeHexString;
}

/**
 * Check whether an account associated to the specified address exists
 */
export const useAccountExists = ({
  address,
}: UseAccountExistsProps) => {
  const { aptosClient } = useNetworks();

  return useQuery(
    [accountQueryKeys.getAccountExists, address],
    async () => aptosClient.getAccount(address!)
      .then(() => true)
      .catch(() => false),
    {
      enabled: Boolean(aptosClient && address),
    },
  );
};

/**
 * Query coin balance for the specified account
 * @param address account address of the balance to be queried
 * @param options query options
 */
export function useAccountCoinBalance(
  address: string | undefined,
  options?: UseQueryOptions<number>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<number>(
    [accountQueryKeys.getAccountCoinBalance, address],
    async () => aptosClient.getAccountResource(address!, aptosCoinStoreStructTag)
      .then((res: any) => Number(res.data.coin.value))
      .catch((err) => {
        if (err instanceof ApiError && err.status === 404) {
          return 0;
        }
        throw err;
      }),
    {
      enabled: Boolean(address),
      ...options,
    },
  );
}

/**
 * Query sequence number for current account,
 * which is required to BCD-encode a transaction locally.
 * The value is queried lazily the first time `get` is called, and is
 * refetched only when an error occurs, by invalidating the cache or
 * manually refetching.
 */
export function useSequenceNumber() {
  const { aptosAccount } = useActiveAccount();
  const { aptosClient } = useNetworks();
  const accountAddress = aptosAccount.address().hex();
  const queryClient = useQueryClient();

  const queryKey = [accountQueryKeys.getSequenceNumber];

  const { refetch } = useQuery(queryKey, async () => {
    if (!accountAddress) {
      return undefined;
    }
    const account = await aptosClient.getAccount(accountAddress!);
    return BigInt(account.sequence_number);
  }, { enabled: false });

  return {
    get: async () => {
      const value = queryClient.getQueryData<bigint>(queryKey);
      return value !== undefined
        ? value
        : (await refetch({ throwOnError: true })).data!;
    },
    increment: () => queryClient.setQueryData<bigint | undefined>(
      queryKey,
      (prev?: bigint) => prev && prev + BigInt(1),
    ),
    refetch,
  };
}

/**
 * Query stake balance for the specified account
 * @param address account address of the balance to be queried
 * @param options query options
 */
export function useAccountStakeBalance(
  address: string | undefined,
  options?: UseQueryOptions<number>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<number>(
    [accountQueryKeys.getAccountStakeBalance, address],
    async () => aptosClient.getAccountResource(address!, aptosStakePoolStructTag)
      .then((res: any) => Number(res.data.active.value))
      .catch((err) => {
        if (err instanceof ApiError && err.status === 404) {
          return 0;
        }
        throw err;
      }),
    {
      enabled: Boolean(address),
      ...options,
    },
  );
}

interface StakeInfo {
  delegatedVoter: MaybeHexString;
  locked_until_secs: string;
  operatorAddress: MaybeHexString;
  value: number;
}

/**
 * Query stake info for the specified account
 * @param address account address of the balance to be queried
 * @param options query options
 * @returns {StakeInfo}
 */
export function useAccountStakeInfo(
  address: string | undefined,
  options?: UseQueryOptions<StakeInfo>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<StakeInfo>(
    [accountQueryKeys.getAccountStakeInfo, address],
    async () => aptosClient.getAccountResource(address!, aptosStakePoolStructTag)
      .then((res: any) => ({
        delegatedVoter: res.data.delegated_voter,
        locked_until_secs: res.data.locked_until_secs,
        operatorAddress: res.data.operator_address,
        value: Number(res.data.active.value),
      }))
      .catch((err) => {
        if (err instanceof ApiError && err.status === 404) {
          throw err;
        }
        throw err;
      }),
    {
      enabled: Boolean(address),
      ...options,
    },
  );
}
