// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { usePersistentStorageState } from 'core/hooks/useStorageState';
import { useMemo } from 'react';
import { AptosClient, FaucetClient } from 'aptos';
import { WALLET_STATE_CUSTOM_NETWORKS_STORAGE_KEY, WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY } from 'core/constants';
import { ProviderEvent, sendProviderEvent } from 'core/utils/providerEvents';

export enum DefaultNetworks {
  Devnet = 'Devnet',
  Localhost = 'Localhost',
  Testnet = 'Testnet',
}

export interface Network {
  faucetUrl?: string;
  name: string,
  nodeUrl: string;
}

export type Networks = Record<string, Network>;

export const defaultCustomNetworks: Networks = {
  [DefaultNetworks.Localhost]: {
    faucetUrl: 'http://localhost:80',
    name: DefaultNetworks.Localhost,
    nodeUrl: 'http://localhost:8080',
  },
};

export const defaultNetworks: Networks = Object.freeze({
  [DefaultNetworks.Devnet]: {
    faucetUrl: 'https://faucet.devnet.aptoslabs.com',
    name: DefaultNetworks.Devnet,
    nodeUrl: 'https://fullnode.devnet.aptoslabs.com',
  },
  [DefaultNetworks.Testnet]: {
    faucetUrl: undefined,
    name: DefaultNetworks.Testnet,
    nodeUrl: 'https://ait3.aptosdev.com/',
  },
} as const);

export const defaultNetworkName = DefaultNetworks.Devnet;

export default function useNetworks() {
  const [
    activeNetworkName,
    setActiveNetworkName,
    isActiveNetworkNameReady,
  ] = usePersistentStorageState<string>(
    WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
    defaultNetworkName,
  );
  const [
    customNetworks,
    setCustomNetworks,
    areCustomNetworksReady,
  ] = usePersistentStorageState<Networks>(
    WALLET_STATE_CUSTOM_NETWORKS_STORAGE_KEY,
    defaultCustomNetworks,
  );

  const networks = customNetworks
    ? { ...defaultNetworks, ...customNetworks }
    : undefined;

  const activeNetwork = (activeNetworkName && networks)
    ? networks[activeNetworkName]
    : undefined;

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

  const addNetwork = async (network: Network) => {
    const newCustomNetworks = { ...customNetworks, [network.name]: network };
    await setCustomNetworks(newCustomNetworks);
  };

  const removeNetwork = async (networkName: string) => {
    const newCustomNetworks = { ...customNetworks };
    delete newCustomNetworks[networkName];
    await setCustomNetworks(newCustomNetworks);

    if (networkName === activeNetworkName) {
      const firstAvailableNetworkName = Object.keys(newCustomNetworks)[0];
      await setActiveNetworkName(firstAvailableNetworkName);
      await sendProviderEvent(ProviderEvent.NETWORK_CHANGED);
    }
  };

  const switchNetwork = async (networkName: string) => {
    await setActiveNetworkName(networkName);
    await sendProviderEvent(ProviderEvent.NETWORK_CHANGED);
  };

  return {
    activeNetwork,
    activeNetworkName,
    addNetwork,
    aptosClient,
    areNetworksReady: areCustomNetworksReady && isActiveNetworkNameReady,
    faucetClient,
    networks,
    removeNetwork,
    switchNetwork,
  };
}
