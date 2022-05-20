// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY } from 'core/constants';

export type AptosNetwork = 'http://0.0.0.0:8080' | 'https://fullnode.devnet.aptoslabs.com';

export function getLocalStorageNetworkState(): AptosNetwork | null {
  // Get network from local storage by key
  return (window.localStorage.getItem(
    WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
  ) as AptosNetwork | null);
}
