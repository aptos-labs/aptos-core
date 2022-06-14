// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosClient, MaybeHexString, Types,
} from 'aptos';
import { AptosAccountState } from 'core/types';
import { AptosNetwork } from 'core/utils/network';

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

export const getAccountBalanceFromAccountResources = (
  accountResources: Types.AccountResource[] | undefined,
): Number => {
  if (accountResources) {
    const accountResource = (accountResources) ? accountResources?.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>') : undefined;
    const tokenBalance = (accountResource)
      ? (accountResource.data as { coin: { value: string } }).coin.value
      : undefined;
    return Number(tokenBalance);
  }
  return -1;
};

export interface GetResourceFromAccountResources {
  accountResources: Types.AccountResource[] | undefined;
  resource: string
}

export const getResourceFromAccountResources = ({
  accountResources,
  resource,
}: GetResourceFromAccountResources) => ((accountResources)
  ? accountResources?.find((r) => r.type === resource)
  : undefined);

export type GetTestCoinTokenBalanceFromAccountResourcesProps = Omit<GetResourceFromAccountResources, 'resource'>;

export const getTestCoinTokenBalanceFromAccountResources = ({
  accountResources,
}: GetTestCoinTokenBalanceFromAccountResourcesProps) => {
  const testCoinResource = getResourceFromAccountResources({
    accountResources,
    resource: '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>',
  });
  const tokenBalance = (testCoinResource)
    ? (testCoinResource.data as { coin: { value: string } }).coin.value
    : undefined;
  return tokenBalance;
};

export const getAccountExists = async ({
  address,
  nodeUrl,
}: GetAccountResourcesProps) => {
  const client = new AptosClient(nodeUrl);
  try {
    if (address) {
      const account = await client.getAccount(address);
      return !!(account);
    }
  } catch (err) {
    return false;
  }
  return false;
};

interface GetToAddressAccountExistsProps {
  queryKey: (string | {
    aptosAccount: AptosAccountState;
    nodeUrl: AptosNetwork;
    toAddress?: MaybeHexString | null;
  })[]
}

export const getToAddressAccountExists = async (
  { queryKey }: GetToAddressAccountExistsProps,
) => {
  const [, paramsObject] = queryKey;
  if (typeof paramsObject === 'string') return false;
  const { aptosAccount, nodeUrl, toAddress } = paramsObject;
  if (toAddress && aptosAccount) {
    const doesAccountExist = await getAccountExists({ address: toAddress, nodeUrl });
    return doesAccountExist;
  }
  return false;
};
