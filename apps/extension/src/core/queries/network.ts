// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, FaucetClient } from 'aptos';
import { useQuery, UseQueryOptions } from 'react-query';
import { useNetworks } from 'core/hooks/useNetworks';
import { useActiveAccount } from 'core/hooks/useAccounts';

export const networkQueryKeys = Object.freeze({
  getChainId: 'getChainId',
  getFaucetStatus: 'getFaucetStatus',
  getNodeStatus: 'getNodeStatus',
} as const);

async function getIsNodeAvailable(nodeUrl: string) {
  const aptosClient = new AptosClient(nodeUrl);
  try {
    await aptosClient.getLedgerInfo();
    return true;
  } catch {
    return false;
  }
}

interface GetIsFaucetAvailableParams {
  address: string,
  faucetUrl: string,
  nodeUrl: string,
}

/**
 * The only way to consistently know whether the faucet is
 * available is to call the `/mint` endpoint, which requires an account address.
 * Using the active account address is preferred.
 * @param address
 * @param faucetUrl
 * @param nodeUrl
 */
async function getIsFaucetAvailable({
  address,
  faucetUrl,
  nodeUrl,
}: GetIsFaucetAvailableParams) {
  const faucetClient = new FaucetClient(nodeUrl, faucetUrl);
  try {
    // Note: since we're funding 0 coins, the request is fast (no need to wait for transactions)
    const txns = await faucetClient.fundAccount(address, 0);
    return txns.length === 0;
  } catch (err) {
    return false;
  }
}

export function useNodeStatus(
  nodeUrl: string | undefined,
  options?: UseQueryOptions<boolean>,
) {
  const { data, ...rest } = useQuery<boolean>(
    [networkQueryKeys.getNodeStatus, nodeUrl],
    async () => getIsNodeAvailable(nodeUrl!),
    {
      ...options,
      enabled: Boolean(nodeUrl) && options?.enabled,
    },
  );
  return { isNodeAvailable: data, ...rest };
}

export interface UseFaucetStatusProps {
  faucetUrl: string | undefined,
  nodeUrl: string | undefined
}

export function useFaucetStatus(
  { faucetUrl, nodeUrl }: UseFaucetStatusProps,
  options?: UseQueryOptions<boolean>,
) {
  const { activeAccountAddress } = useActiveAccount();
  const { data, ...rest } = useQuery<boolean>(
    [networkQueryKeys.getFaucetStatus, faucetUrl],
    async () => getIsFaucetAvailable({
      address: activeAccountAddress!,
      faucetUrl: faucetUrl!,
      nodeUrl: nodeUrl!,
    }),
    {
      ...options,
      enabled: Boolean(nodeUrl && faucetUrl && activeAccountAddress) && options?.enabled,
    },
  );
  return { isFaucetAvailable: data, ...rest };
}

/**
 * Query chain id associated with the current node,
 * which is required to BCD-encode a transaction locally
 */
export function useChainId() {
  const { aptosClient } = useNetworks();

  return useQuery(
    [networkQueryKeys.getChainId],
    () => aptosClient.getChainId(),
    {
      enabled: Boolean(aptosClient),
      staleTime: 60000,
    },
  );
}
