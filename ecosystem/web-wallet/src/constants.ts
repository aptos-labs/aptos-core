// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export const KEY_LENGTH: number = 64
export const WALLET_STATE_LOCAL_STORAGE_KEY = 'aptosWalletState'

export const STATIC_GAS_AMOUNT = 150

export const LOCAL_NODE_URL = 'http://0.0.0.0:8080'
export const DEVNET_NODE_URL = 'https://fullnode.devnet.aptoslabs.com'
export const LOCAL_FAUCET_URL = 'http://0.0.0.0:8000'
export const DEVNET_FAUCET_URL = 'https://faucet.devnet.aptoslabs.com'

export const NODE_URL = DEVNET_NODE_URL
export const FAUCET_URL = DEVNET_FAUCET_URL

export const secondaryBgColor = {
  dark: 'gray.900',
  light: 'white'
}

export const secondaryErrorMessageColor = {
  dark: 'red.200',
  light: 'red.500'
}

export const validStorageUris = [
  'amazonaws.com',
  'ipfs.io',
  'arweave.net'
]
