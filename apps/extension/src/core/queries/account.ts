// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, MaybeHexString } from 'aptos';
import { useQuery, UseQueryOptions } from 'react-query';
import { aptosCoinStoreStructTag, aptosStakePoolStructTag } from 'core/constants';
import { useNetworks } from 'core/hooks/useNetworks';
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
    async () => aptosClient.getAccountResource(address!, aptosStakePoolStructTag)
      .then((res: any) => ({
        delegatedVoter: res.data.delegated_voter,
        lockedUntilSecs: res.data.locked_until_secs,
        operatorAddress: res.data.operator_address,
        value: Number(res.data.active.value),
      }))
      .catch((err) => {
        if (err instanceof ApiError && err.status === 404) {
          return undefined;
        }
        throw err;
      }),
    {
      enabled: Boolean(address),
      ...options,
    },
  );
}
