// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY } from 'core/constants';

export enum NetworkType {
  DEVNET = 'Devnet',
  LOCALHOST = 'Localhost',
  TESTNET = 'Testnet',
}

export const defaultNetworkType = NetworkType.DEVNET;

export const nodeUrlMap = Object.freeze({
  [NetworkType.DEVNET]: 'https://fullnode.devnet.aptoslabs.com',
  [NetworkType.LOCALHOST]: 'http://0.0.0.0:8080',
  [NetworkType.TESTNET]: 'https://ait3.aptosdev.com/',
} as const);

export const nodeUrlReverseMap = Object.freeze({
  [nodeUrlMap.Localhost]: NetworkType.LOCALHOST,
  [nodeUrlMap.Devnet]: NetworkType.DEVNET,
  [nodeUrlMap.Testnet]: NetworkType.TESTNET,
} as const);

export const faucetUrlMap = Object.freeze({
  [NetworkType.DEVNET]: 'https://faucet.devnet.aptoslabs.com',
  [NetworkType.LOCALHOST]: 'http://0.0.0.0:8000',
  [NetworkType.TESTNET]: 'https://faucet.devnet.aptoslabs.com',
} as const);

export const faucetUrlReverseMap = Object.freeze({
  [faucetUrlMap.Localhost]: NetworkType.LOCALHOST,
  [faucetUrlMap.Devnet]: NetworkType.DEVNET,
  [faucetUrlMap.Testnet]: NetworkType.TESTNET,
});

export type NodeUrl = typeof nodeUrlMap[keyof typeof nodeUrlMap];
export type FaucetUrl = typeof faucetUrlMap[keyof typeof faucetUrlMap];

export function getLocalStorageNodeNetworkUrl(): NodeUrl {
  // Get network from local storage by key
  return (window.localStorage.getItem(
    WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
  ) as NodeUrl | null) || nodeUrlMap[defaultNetworkType];
}

export function getFaucetUrlFromNodeUrl(
  nodeNetworkUrl: NodeUrl,
): FaucetUrl {
  return faucetUrlMap[nodeUrlReverseMap[nodeNetworkUrl]];
}
