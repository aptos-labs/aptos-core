// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosClient, MaybeHexString,
} from 'aptos';
import useWalletState from 'core/hooks/useWalletState';
import { useCallback } from 'react';
import { useQuery } from 'react-query';
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
  const { aptosNetwork: nodeUrl } = useWalletState();

  return useQuery(
    [accountQueryKeys.getAccountExists, address],
    async () => getAccountExists({ address: address!, nodeUrl }),
    { enabled: Boolean(address) },
  );
};

interface UseAccountCoinBalanceParams {
  refetchInterval?: number | false,
}

/**
 * Query coin balance for the current account
 * @param refetchInterval automatic refetch interval in milliseconds
 */
export function useAccountCoinBalance({
  refetchInterval,
}: UseAccountCoinBalanceParams = {}) {
  const { aptosAccount, aptosNetwork } = useWalletState();

  const accountAddress = aptosAccount?.address()?.hex();
  return useQuery([accountQueryKeys.getAccountCoinBalance, accountAddress], async () => {
    const client = new AptosClient(aptosNetwork);
    const resource: any = await client.getAccountResource(accountAddress!, aptosCoinStoreStructTag);
    return Number(resource.data.coin.value);
  }, {
    enabled: Boolean(accountAddress),
    refetchInterval,
  });
}

/**
 * Query sequence number for current account,
 * which is required to BCD-encode a transaction locally
 */
export function useSequenceNumber() {
  const walletState = useWalletState();
  const aptosNetwork = walletState.aptosNetwork!;
  const aptosAccount = walletState.aptosAccount!;

  const sequenceNumberQuery = useCallback(async () => {
    const aptosClient = new AptosClient(aptosNetwork);
    return aptosClient.getAccount(aptosAccount.address())
      .then(({ sequence_number }) => Number(sequence_number));
  }, [aptosNetwork, aptosAccount]);

  return useQuery([accountQueryKeys.getSequenceNumber], sequenceNumberQuery);
}
