// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  ApiError,
  MaybeHexString,
} from 'aptos';
import { useQuery, UseQueryOptions } from 'react-query';
import {
  aptosCoinStoreStructTag,
  aptosCoinStructTag,
  aptosStakePoolStructTag,
} from 'core/constants';
import { useNetworks } from 'core/hooks/useNetworks';
import { CoinInfoData } from 'shared/types/resource';
import {
  useFetchAccountResource,
  useFetchAccountResources,
} from 'core/queries/useAccountResources';
import useCachedRestApi from 'core/hooks/useCachedRestApi';
import getCoinStoresByCoinType from 'core/utils/resource';

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
  options?: UseQueryOptions<bigint>,
) {
  const fetchAccountResource = useFetchAccountResource();
  return useQuery<bigint>(
    [accountQueryKeys.getAccountOctaCoinBalance, address],
    async () => fetchAccountResource(address!, aptosCoinStoreStructTag)
      .then((res: any) => BigInt(res.data.coin.value))
      .catch((err) => {
        if (err instanceof ApiError && err.status === 404) {
          return 0n;
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

type AccountCoinResource = {
  balance: bigint;
  info: CoinInfoData,
  type: string;
};

/**
 * Query for all the coins in the user's account
 * @param address account address of the balance to be queried
 * @param options? query options
 */
export function useAccountCoinResources(
  address: string | undefined,
  options?: UseQueryOptions<AccountCoinResource[]>,
) {
  const fetchAccountResources = useFetchAccountResources();
  const { getCoinInfo } = useCachedRestApi();

  return useQuery<AccountCoinResource[]>(
    [accountQueryKeys.getAccountCoinResources, address],
    async () => fetchAccountResources(address!)
      .then(async (resources) => {
        const coinStores = getCoinStoresByCoinType(resources);
        const result: AccountCoinResource[] = [];
        // Extract info for non-empty coin stores
        await Promise.all(Object.entries(coinStores).map(async ([coinType, coinStore]) => {
          const balance = BigInt(coinStore.coin.value);
          const coinInfo = await getCoinInfo(coinType);
          if (balance !== 0n && coinInfo !== undefined) {
            result.push({ balance, info: coinInfo, type: coinType });
          }
        }));

        // Sort by descending balance, with APT always on top
        return result.sort((lhs, rhs) => {
          if (lhs.balance > rhs.balance && rhs.type !== aptosCoinStructTag) return -1;
          if (lhs.balance < rhs.balance && lhs.type !== aptosCoinStructTag) return 1;
          return 0;
        });
      })
      .catch((err) => {
        if (err instanceof ApiError && err.status === 404) {
          return [];
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
  options?: UseQueryOptions<bigint>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<bigint>(
    [accountQueryKeys.getAccountStakeBalance, address],
    async () => aptosClient.getAccountResource(address!, aptosStakePoolStructTag)
      .then((res: any) => BigInt(res.data.active.value))
      .catch((err) => {
        if (err instanceof ApiError && err.status === 404) {
          return 0n;
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
  value: bigint;
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
            value: BigInt(res.data.active.value),
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
