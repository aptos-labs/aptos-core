// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, FaucetClient } from 'aptos';
import constate from 'constate';
import { useMemo } from 'react';
import { useAppState } from 'core/hooks/useAppState';
import { triggerNetworkChange } from 'core/utils/providerEvents';
import {
  defaultCustomNetworks,
  defaultNetworkName,
  defaultNetworks,
  Network,
} from 'shared/types';

/**
 * Hook/provider for accessing and updating the networks state.
 * The set of available networks is the union between `defaultNetworks` (which is constant)
 * and `customNetworks` which is editable by the user
 */
export const [NetworksProvider, useNetworks] = constate(() => {
  const {
    activeAccountAddress,
    updatePersistentState,
    ...appState
  } = useAppState();

  const activeNetworkName = appState.activeNetworkName ?? defaultNetworkName;
  const customNetworks = appState.customNetworks ?? defaultCustomNetworks;

  const networks = { ...defaultNetworks, ...customNetworks };
  const activeNetwork = networks[activeNetworkName] ?? networks[defaultNetworkName];

  const addNetwork = async (network: Network, shouldSwitch: boolean = true) => {
    const newCustomNetworks = { ...customNetworks, [network.name]: network };

    if (shouldSwitch) {
      await updatePersistentState({
        activeNetworkName: network.name,
        customNetworks: newCustomNetworks,
      });
      await triggerNetworkChange(activeAccountAddress, { networkName: network.name });
    } else {
      await updatePersistentState({ customNetworks: newCustomNetworks });
    }
  };

  const editNetwork = async (network: Network) => {
    if (network.name in networks) {
      const newCustomNetworks = { ...customNetworks, [network.name]: network };
      await updatePersistentState({ customNetworks: newCustomNetworks });
    }
  };

  const removeNetwork = async (networkName: string) => {
    const newCustomNetworks = { ...customNetworks };
    delete newCustomNetworks[networkName];

    if (networkName === activeNetworkName) {
      const firstAvailableNetworkName = Object.keys(networks)[0];
      await updatePersistentState({
        activeNetworkName: firstAvailableNetworkName,
        customNetworks: newCustomNetworks,
      });
      await triggerNetworkChange(activeAccountAddress, { networkName: firstAvailableNetworkName });
    } else {
      await updatePersistentState({ customNetworks: newCustomNetworks });
    }
  };

  const switchNetwork = async (networkName: string) => {
    await updatePersistentState({ activeNetworkName: networkName });
    await triggerNetworkChange(activeAccountAddress, { networkName });
  };

  const aptosClient = useMemo(
    () => new AptosClient(activeNetwork.nodeUrl),
    [activeNetwork],
  );

  const faucetClient = useMemo(
    () => (activeNetwork.faucetUrl
      ? new FaucetClient(activeNetwork.nodeUrl, activeNetwork.faucetUrl)
      : undefined),
    [activeNetwork],
  );

  return {
    activeNetwork,
    activeNetworkName,
    addNetwork,
    aptosClient,
    customNetworks,
    editNetwork,
    faucetClient,
    networks,
    removeNetwork,
    switchNetwork,
  };
});
