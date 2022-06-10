// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  DEVNET_NODE_URL,
  LOCAL_NODE_URL,
  WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
  DEVNET_FAUCET_URL,
  LOCAL_FAUCET_URL,
} from 'core/constants';

export type AptosNetwork = 'http://0.0.0.0:8080' | 'https://fullnode.devnet.aptoslabs.com';
export type FaucetNetwork = 'http://0.0.0.0:8000' | 'https://faucet.devnet.aptoslabs.com';

export const networkUriMap: Record<string | number, string> = {
  'http://0.0.0.0:8080': 'Localhost',
  'https://fullnode.devnet.aptoslabs.com': 'Devnet',
};

export const faucetUriMap = Object.freeze({
  DEVNET_NODE_URL: DEVNET_FAUCET_URL,
  LOCAL_NODE_URL: LOCAL_FAUCET_URL,
} as const);

export function getLocalStorageNetworkState(): AptosNetwork {
  // Get network from local storage by key
  return (window.localStorage.getItem(
    WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
  ) as AptosNetwork | null) || DEVNET_NODE_URL;
}

function assertNeverNetwork(x: never): never {
  throw new Error(`Unexpected network: ${x}`);
}

export function getFaucetNetworkFromAptosNetwork(aptosNetwork: AptosNetwork): FaucetNetwork {
  switch (aptosNetwork) {
    case DEVNET_NODE_URL: return faucetUriMap.DEVNET_NODE_URL;
    case LOCAL_NODE_URL: return faucetUriMap.LOCAL_NODE_URL;
    default: return assertNeverNetwork(aptosNetwork);
  }
}
