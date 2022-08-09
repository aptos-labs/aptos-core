// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import axios from 'axios';
import { useQuery } from 'react-query';
import { useWalletState } from 'core/hooks/useWalletState';
import { useCallback } from 'react';
import { AptosClient } from 'aptos';
import { faucetUrlMap, nodeUrlMap } from 'core/utils/network';

export const getLocalhostIsLive = async () => {
  try {
    const localNode = axios.get(nodeUrlMap.Localhost);
    const localFaucet = axios.get(faucetUrlMap.Localhost);
    const localHostIsLive = await Promise.all(
      [localNode, localFaucet],
    ).then(([localNodeValue, localFaucetValue]) => (
      localNodeValue.status === 200 && localFaucetValue.status === 200
    ));
    return localHostIsLive;
  } catch (err: any) {
    // TODO, this MUST be changed in the future, currently there are CORS issues
    // on faucet and its difficult to tell if the faucet port is live. Current
    // behavior is that it just assumes its live if localFaucet returns an error.
    // Should be fixed so that CORS errors are eliminated and we can accurately
    // tell if the network is live or not
    if (err.config.url === faucetUrlMap.Localhost) {
      return true;
    }
    return false;
  }
};

export const networkQueryKeys = Object.freeze({
  getChainId: 'getChainId',
  getTestnetStatus: 'getTestnetStatus',
} as const);

export const useTestnetStatus = () => useQuery(
  networkQueryKeys.getTestnetStatus,
  getLocalhostIsLive,
  { refetchInterval: 1000 },
);

/**
 * Query chain id associated with the current node,
 * which is required to BCD-encode a transaction locally
 */
export function useChainId() {
  const walletState = useWalletState();
  const NodeNetworkUrl = walletState.nodeUrl!;

  const chainIdQuery = useCallback(async () => {
    const aptosClient = new AptosClient(NodeNetworkUrl);
    return aptosClient.getChainId();
  }, [NodeNetworkUrl]);

  return useQuery([networkQueryKeys.getChainId], chainIdQuery, {
    staleTime: 60000,
  });
}

export default getLocalhostIsLive;
