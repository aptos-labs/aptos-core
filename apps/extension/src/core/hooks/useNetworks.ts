// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { usePersistentStorageState } from 'core/hooks/useStorageState';
import { useMemo } from 'react';
import { AptosClient, FaucetClient } from 'aptos';
import { WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY } from 'core/constants';
import { ProviderEvent, sendProviderEvent } from 'core/utils/providerEvents';

export enum NetworkType {
  Devnet = 'devnet',
  LocalHost = 'localhost',
  Testnet = 'testnet',
}

export interface Network {
  faucetUrl?: string;
  name: string,
  nodeUrl: string;
}

export const defaultNetworks = Object.freeze({
  [NetworkType.Devnet]: {
    faucetUrl: 'https://faucet.devnet.aptoslabs.com',
    name: 'Devnet',
    nodeUrl: 'https://fullnode.devnet.aptoslabs.com',
  },
  [NetworkType.LocalHost]: {
    faucetUrl: 'http://0.0.0.0:8000',
    name: 'Localhost',
    nodeUrl: 'http://0.0.0.0:8080',
  },
  [NetworkType.Testnet]: {
    faucetUrl: undefined,
    name: 'Testnet',
    nodeUrl: 'https://ait3.aptosdev.com/',
  },
} as const);

export const defaultNetworkType = NetworkType.Devnet;

export default function useNetworks() {
  const [
    activeNetworkType,
    setActiveNetworkType,
    isNetworkTypeReady,
  ] = usePersistentStorageState<NetworkType>(
    WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
    defaultNetworkType,
  );

  const activeNetwork = activeNetworkType ? defaultNetworks[activeNetworkType] : undefined;

  const aptosClient = useMemo(
    () => (activeNetwork ? new AptosClient(activeNetwork.nodeUrl) : undefined),
    [activeNetwork],
  );

  const faucetClient = useMemo(
    () => (activeNetwork && activeNetwork.faucetUrl
      ? new FaucetClient(activeNetwork.nodeUrl, activeNetwork.faucetUrl)
      : undefined),
    [activeNetwork],
  );

  const switchNetwork = async (network: NetworkType) => {
    await setActiveNetworkType(network);
    await sendProviderEvent(ProviderEvent.NETWORK_CHANGED);
  };

  return {
    activeNetwork,
    activeNetworkType,
    aptosClient,
    areNetworksReady: isNetworkTypeReady,
    faucetClient,
    networks: defaultNetworks,
    switchNetwork,
  };
}
