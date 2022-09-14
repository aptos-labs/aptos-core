// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { ApiError, AptosClient, MaybeHexString } from 'aptos';
import { useQuery, UseQueryOptions } from 'react-query';
import { aptosCoinStoreStructTag, aptosStakePoolStructTag, coinStoreStructTag } from 'core/constants';
import { useNetworks } from 'core/hooks/useNetworks';

/**
 * QUERY KEYS
 */
export const accountQueryKeys = Object.freeze({
  getAccountCoinResources: 'getAccountCoinResources',
  getAccountExists: 'getAccountExists',
  getAccountOctaCoinBalance: 'getAccountOctaCoinBalance',
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
 * Query coin balance for the specified account in Octa -> APT * 10^-8
 * @param address account address of the balance to be queried
 * @param options? query options
 */
export function useAccountOctaCoinBalance(
  address: string | undefined,
  options?: UseQueryOptions<number>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<number>(
    [accountQueryKeys.getAccountOctaCoinBalance, address],
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
      retry: 0,
      ...options,
    },
  );
}

type AccountCoinResource = { type: string; value: number; };

/**
 * Query for all the coins in the user's account
 * @param address account address of the balance to be queried
 * @param options? query options
 */
export function useAccountCoinResources(
  address: string | undefined,
  options?: UseQueryOptions<AccountCoinResource[]>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<AccountCoinResource[]>(
    [accountQueryKeys.getAccountCoinResources, address],
    async () => aptosClient.getAccountResources(address!)
      .then((res: any[]) => {
        const result: AccountCoinResource[] = [];
        res.forEach((item) => {
          if (item.type.includes(coinStoreStructTag)) {
            result.push({
              type: item.type,
              value: Number(item.data.coin.value),
            });
          }
        });
        return result;
      })
      .catch((err) => {
        if (err instanceof ApiError && err.status === 404) {
          const emptyArray: AccountCoinResource[] = [];
          return emptyArray;
        }
        throw err;
      }),
    {
      enabled: Boolean(address),
      retry: 0,
      ...options,
    },
  );
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
      retry: 0,
      ...options,
    },
  );
}

export interface StakeInfo {
  delegatedVoter: MaybeHexString;
  lockedUntilSecs: string;
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
  options?: UseQueryOptions<StakeInfo | undefined>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<StakeInfo | undefined>(
    [accountQueryKeys.getAccountStakeInfo, address],
    async () => {
      try {
        return await aptosClient.getAccountResource(address!, aptosStakePoolStructTag)
          .then((res: any) => ({
            delegatedVoter: res.data.delegated_voter,
            lockedUntilSecs: res.data.locked_until_secs,
            operatorAddress: res.data.operator_address,
            value: Number(res.data.active.value),
          } as StakeInfo));
      } catch (err) {
        if (err instanceof ApiError && err.status === 404) {
          return undefined;
        }
        throw err;
      }
    },
    {
      enabled: Boolean(address),
      retry: 0,
      ...options,
    },
  );
}
