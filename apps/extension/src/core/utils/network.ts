// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY } from 'core/constants';

export enum NetworkType {
  DEVNET = 'Devnet',
  LOCALHOST = 'Localhost',
}

export const nodeUrlMap = Object.freeze({
  [NetworkType.DEVNET]: 'https://fullnode.devnet.aptoslabs.com',
  [NetworkType.LOCALHOST]: 'http://0.0.0.0:8080',
} as const);

export const nodeUrlReverseMap = Object.freeze({
  [nodeUrlMap.Localhost]: NetworkType.LOCALHOST,
  [nodeUrlMap.Devnet]: NetworkType.DEVNET,
} as const);

export const faucetUrlMap = Object.freeze({
  [NetworkType.DEVNET]: 'https://faucet.devnet.aptoslabs.com',
  [NetworkType.LOCALHOST]: 'http://0.0.0.0:8000',
} as const);

export const faucetUrlReverseMap = Object.freeze({
  [faucetUrlMap.Localhost]: NetworkType.LOCALHOST,
  [faucetUrlMap.Devnet]: NetworkType.DEVNET,
});

export type NodeUrl = typeof nodeUrlMap[keyof typeof nodeUrlMap];
export type FaucetUrl = typeof faucetUrlMap[keyof typeof faucetUrlMap];

export function getLocalStorageNodeNetworkUrl(): NodeUrl {
  // Get network from local storage by key
  return (window.localStorage.getItem(
    WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
  ) as NodeUrl | null) || nodeUrlMap.Devnet;
}

export function getFaucetUrlFromNodeUrl(
  NodeNetworkUrlUrl: NodeUrl,
): FaucetUrl {
  return faucetUrlMap[nodeUrlReverseMap[NodeNetworkUrlUrl]];
}
