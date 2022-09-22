// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  ApiError,
  AptosClient,
  MaybeHexString,
} from 'aptos';
import { useQuery, UseQueryOptions } from 'react-query';
import {
  aptosCoinStoreStructTag,
  aptosStakePoolStructTag,
  coinInfoResource,
  coinStoreResource,
  coinStoreStructTag,
} from 'core/constants';
import { useNetworks } from 'core/hooks/useNetworks';
import { CoinInfoData } from 'shared/types/resource';

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
  const { aptosClient } = useNetworks();

  return useQuery<bigint>(
    [accountQueryKeys.getAccountOctaCoinBalance, address],
    async () => aptosClient.getAccountResource(address!, aptosCoinStoreStructTag)
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

export const replaceCoinStoreWithCoinInfo = (structTag: string) => {
  const replaceString = structTag.replace(coinStoreResource, coinInfoResource);
  return replaceString;
};

/**
 * @summary Returns a dictionary with the parsed coin info from the struct tag
 * @example
 * ```ts
 *  const {
 *    address,
 *    resource
 *  } = parseCoinInfoStructTag(
 *    '0x1::coin::CoinInfo<0x1::aptos_coin::AptosCoin>'
 *  )
 * ```
 */
export const parseCoinInfoStructTag = (coinInfoStructTag: string) => {
  const address = coinInfoStructTag.toString().split('::')[2].split('<')[1];
  const resource = coinInfoStructTag.toString().split('<')[1].replace('>', '');
  return {
    address,
    resource,
  };
};

interface GetCoinInfoParams {
  accountAddress: MaybeHexString;
  coinInfoStructTag: string;
  nodeUrl: string;
}

/**
 * @summary Gets CoinInfo from an address that holds the CoinInfo (ie. the creator account)
 * @see https://fullnode.devnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::coin::CoinInfo%3C0x1::aptos_coin::AptosCoin%3E
 * @description in the url above:
 *              %3C = "<" character for the opening generic bracket
 *              %3E = ">" character for the closing generic bracket
 */
export const getCoinInfo = async ({
  accountAddress,
  coinInfoStructTag,
  nodeUrl,
}: GetCoinInfoParams) => {
  const aptosClient = new AptosClient(nodeUrl);
  const coinInfo = await aptosClient.getAccountResource(accountAddress, coinInfoStructTag);
  const coinInfoData = coinInfo.data as CoinInfoData;
  const { decimals, name, symbol } = coinInfoData;

  return ({
    decimals,
    name,
    symbol,
  });
};

type AccountCoinResource = {
  coinInfoAddress: string;
  coinInfoStructTag: string;
  decimals: number;
  name: string;
  symbol: string;
  type: string;
  value: bigint;
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
  const { activeNetwork, aptosClient } = useNetworks();

  return useQuery<AccountCoinResource[]>(
    [accountQueryKeys.getAccountCoinResources, address],
    async () => aptosClient.getAccountResources(address!)
      .then(async (res: any[]) => {
        const result: Omit<AccountCoinResource, 'decimals' | 'name' | 'symbol'>[] = [];
        res.forEach((item) => {
          if (item.type.includes(coinStoreStructTag)) {
            const { type } = item;
            const coinInfoStructTag = replaceCoinStoreWithCoinInfo(type);
            const { address: coinInfoAddress } = parseCoinInfoStructTag(coinInfoStructTag);
            result.push({
              coinInfoAddress,
              coinInfoStructTag,
              type: item.type,
              value: BigInt(item.data.coin.value),
            });
          }
        });

        const finalCoinInfo = await Promise.all((result.map(async (item) => {
          const coinInfo = await getCoinInfo({
            accountAddress: item.coinInfoAddress,
            coinInfoStructTag: item.coinInfoStructTag,
            nodeUrl: activeNetwork.nodeUrl,
          });

          return {
            ...item,
            ...coinInfo,
          };
        })));

        return finalCoinInfo;
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
