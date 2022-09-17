// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { MaybeHexString } from 'aptos';

interface EventSchema {
  [eventType: string]: {
    action: string;
    category: string;
    label?: string;
  }
}

const eventSchemaTypeCheck = <T extends EventSchema>(o: T) => o;

/**
 * @summary Should be tied to a page or module
 */
export const analyticsCategories = Object.freeze({
  ACCOUNT: 'Account',
  COIN: 'Coin',
  COLLECTIBLES: 'Collectibles',
  FAUCET: 'Faucet',
  SETTINGS: 'Settings',
  TRANSACTION: 'Transaction',
} as const);

/**
 *     /\                            | |
 *    /  \   ___ ___ ___  _   _ _ __ | |_
 *   / /\ \ / __/ __/ _ \| | | | '_ \| __|
 *  / ____ \ (_| (_| (_) | |_| | | | | |_
 * /_/    \_\___\___\___/ \__,_|_| |_|\__|
 *
 * Account Analytics Events
 */

const accountActions = Object.freeze({
  CREATE_ACCOUNT: 'Create account',
  IMPORT_ACCOUNT: 'Import account',
  LOGIN_WITH_PRIVATE_KEY: 'Login with private key',
  REMOVE_ACCOUNT: 'Remove account',
  SIGN_OUT: 'Sign out',
  SWITCH_ACCOUNT: 'Switch account',
} as const);

const accountLabels = Object.freeze({
  CREATE_ACCOUNT: 'Create account',
  IMPORT_ACCOUNT: 'Import account',
  LOGIN_WITH_PRIVATE_KEY: 'Login',
  REMOVE_ACCOUNT: 'Remove account',
  SIGN_OUT: 'Sign out',
  SWITCH_ACCOUNT: 'Switch account',
} as const);

export const loginEvents = eventSchemaTypeCheck({
  ERROR_LOGIN_WITH_PRIVATE_KEY: {
    action: `${accountActions.LOGIN_WITH_PRIVATE_KEY} - invalid private key` as const,
    category: analyticsCategories.ACCOUNT,
    label: `${accountLabels.LOGIN_WITH_PRIVATE_KEY} error` as const,
  },
  LOGIN_WITH_PRIVATE_KEY: {
    action: accountActions.LOGIN_WITH_PRIVATE_KEY,
    category: analyticsCategories.ACCOUNT,
    label: accountLabels.LOGIN_WITH_PRIVATE_KEY,
  },
} as const);

export const createAccountEvents = eventSchemaTypeCheck({
  CREATE_ACCOUNT: {
    action: accountActions.CREATE_ACCOUNT,
    category: analyticsCategories.ACCOUNT,
    label: accountLabels.CREATE_ACCOUNT,
  },
  ERROR_CREATE_ACCOUNT: {
    action: `${accountActions.CREATE_ACCOUNT} - unable to create account` as const,
    category: analyticsCategories.ACCOUNT,
    label: `${accountLabels.CREATE_ACCOUNT} error` as const,
  },
} as const);

export const importAccountEvents = eventSchemaTypeCheck({
  ERROR_IMPORT_MNEMONIC_ACCOUNT: {
    action: `${accountActions.IMPORT_ACCOUNT} - unable to import mnemonic` as const,
    category: analyticsCategories.ACCOUNT,
    label: `${accountLabels.IMPORT_ACCOUNT} mnemonic error` as const,
  },
  ERROR_IMPORT_PK_ACCOUNT: {
    action: `${accountActions.IMPORT_ACCOUNT} - unable to import private key` as const,
    category: analyticsCategories.ACCOUNT,
    label: `${accountLabels.IMPORT_ACCOUNT} private key error` as const,
  },
  IMPORT_MNEMONIC_ACCOUNT: {
    action: `${accountActions.IMPORT_ACCOUNT} - mnemonic` as const,
    category: analyticsCategories.ACCOUNT,
    label: `${accountLabels.IMPORT_ACCOUNT} mnemonic` as const,
  },
  IMPORT_PK_ACCOUNT: {
    action: `${accountActions.IMPORT_ACCOUNT} - private key` as const,
    category: analyticsCategories.ACCOUNT,
    label: `${accountLabels.IMPORT_ACCOUNT} private key` as const,
  },
} as const);

export const switchAccountEvents = eventSchemaTypeCheck({
  ERROR_SWITCHING_ACCOUNT: {
    action: `${accountActions.SWITCH_ACCOUNT} - unable to switch account` as const,
    category: analyticsCategories.ACCOUNT,
    label: `${accountLabels.SWITCH_ACCOUNT} error` as const,
  },
  SWITCH_ACCOUNT: {
    action: accountActions.SWITCH_ACCOUNT,
    category: analyticsCategories.ACCOUNT,
    label: accountLabels.SWITCH_ACCOUNT,
  },
} as const);

export const removeAccountEvents = eventSchemaTypeCheck({
  ERROR_REMOVE_ACCOUNT: {
    action: `${accountActions.REMOVE_ACCOUNT} - unable to remove account` as const,
    category: analyticsCategories.ACCOUNT,
    label: `${accountLabels.REMOVE_ACCOUNT} error` as const,
  },
  REMOVE_ACCOUNT: {
    action: accountActions.REMOVE_ACCOUNT,
    category: analyticsCategories.ACCOUNT,
    label: accountLabels.REMOVE_ACCOUNT,
  },
});

export const signOutEvents = eventSchemaTypeCheck({
  SIGN_OUT: {
    action: accountActions.SIGN_OUT,
    category: analyticsCategories.ACCOUNT,
    label: accountLabels.SIGN_OUT,
  },
} as const);

export const accountEvents = eventSchemaTypeCheck({
  ...loginEvents,
  ...createAccountEvents,
  ...signOutEvents,
  ...switchAccountEvents,
  ...importAccountEvents,
  ...removeAccountEvents,
} as const);

export interface AccountEventParams {
  address?: MaybeHexString;
}

/**
 *    _____      _
 *  / ____|    (_)
 * | |     ___  _ _ __
 * | |    / _ \| | '_ \
 * | |___| (_) | | | | |
 *  \_____\___/|_|_| |_|
 *
 * Coin Analytics Events
 */

const coinActions = Object.freeze({
  TRANSFER_COIN: 'Transfer coin',
} as const);

const coinLabels = Object.freeze({
  TRANSFER_APTOS_COIN: 'Transfer Aptos coin',
  TRANSFER_COIN: 'Transfer coin',
} as const);

export const coinEvents = eventSchemaTypeCheck({
  ERROR_TRANSFER_APTOS_COIN: {
    action: `${coinActions.TRANSFER_COIN} - unable to transfer coin` as const,
    category: analyticsCategories.COIN,
    label: `${coinLabels.TRANSFER_APTOS_COIN} error` as const,
  },
  TRANSFER_APTOS_COIN: {
    action: coinActions.TRANSFER_COIN,
    category: analyticsCategories.COIN,
    label: coinLabels.TRANSFER_APTOS_COIN,
  },
  TRANSFER_COIN: {
    action: coinActions.TRANSFER_COIN,
    category: analyticsCategories.COIN,
    label: coinLabels.TRANSFER_COIN,
  },
} as const);

export interface CoinEventParams {
  amount?: number;
  coinType?: string;
  fromAddress?: MaybeHexString;
  toAddress?: MaybeHexString;
}

/**
 *    _____      _ _           _   _ _     _
 *  / ____|    | | |         | | (_) |   | |
 *  | |     ___ | | | ___  ___| |_ _| |__ | | ___  ___
 *  | |    / _ \| | |/ _ \/ __| __| | '_ \| |/ _ \/ __|
 *  | |___| (_) | | |  __/ (__| |_| | |_) | |  __/\__ \
 *   \_____\___/|_|_|\___|\___|\__|_|_.__/|_|\___||___/
 *
 * Collectibles Analytics Events
 */

const collectiblesActions = Object.freeze({
  CLAIM_TOKEN: 'Transfer token',
  CREATE_TOKEN: 'Create token',
  OFFER_TOKEN: 'Offer token',
} as const);

const collectiblesLabels = Object.freeze({
  CLAIM_NFT: 'Claim NFT',
  CLAIM_TOKEN: 'Claim token',
  CREATE_NFT: 'Create NFT',
  CREATE_TOKEN: 'Create token',
  OFFER_NFT: 'Offer NFT',
  OFFER_TOKEN: 'Offer token',
} as const);

export const collectiblesEvents = eventSchemaTypeCheck({
  CLAIM_NFT: {
    action: collectiblesActions.CLAIM_TOKEN,
    category: analyticsCategories.COLLECTIBLES,
    label: collectiblesLabels.CLAIM_NFT,
  },
  CLAIM_TOKEN: {
    action: collectiblesActions.CLAIM_TOKEN,
    category: analyticsCategories.COLLECTIBLES,
    label: collectiblesLabels.CLAIM_TOKEN,
  },
  CREATE_NFT: {
    action: collectiblesActions.CREATE_TOKEN,
    category: analyticsCategories.COLLECTIBLES,
    label: collectiblesLabels.CREATE_NFT,
  },
  CREATE_TOKEN: {
    action: collectiblesActions.CREATE_TOKEN,
    category: analyticsCategories.COLLECTIBLES,
    label: collectiblesLabels.CREATE_TOKEN,
  },
  ERROR_CLAIM_TOKEN: {
    action: `${collectiblesActions.CLAIM_TOKEN} - unable to claim token` as const,
    category: analyticsCategories.COLLECTIBLES,
    label: `${collectiblesLabels.CLAIM_TOKEN} error` as const,
  },
  OFFER_NFT: {
    action: collectiblesActions.OFFER_TOKEN,
    category: analyticsCategories.COLLECTIBLES,
    label: collectiblesLabels.OFFER_NFT,
  },
  OFFER_TOKEN: {
    action: collectiblesActions.OFFER_TOKEN,
    category: analyticsCategories.COLLECTIBLES,
    label: collectiblesLabels.OFFER_TOKEN,
  },
} as const);

export interface CollectibleEventParams {
  amount?: number;
  collectionName?: string;
  description?: string;
  fromAddress?: MaybeHexString;
  name?: string;
  toAddress?: MaybeHexString;
  uri?: string;
}

/**
 *   ______                   _
 * |  ____|                 | |
 * | |__ __ _ _   _  ___ ___| |_
 * |  __/ _` | | | |/ __/ _ \ __|
 * | | | (_| | |_| | (_|  __/ |_
 * |_|  \__,_|\__,_|\___\___|\__|
 *
 * Faucet Analytics Events
 */

const faucetActions = Object.freeze({
  CLICK_FAUCET: 'Click faucet',
} as const);

export const faucetEvents = eventSchemaTypeCheck({
  ERROR_RECEIVE_FAUCET: {
    action: `${faucetActions.CLICK_FAUCET} - unable to query faucet` as const,
    category: analyticsCategories.FAUCET,
    label: `${faucetActions.CLICK_FAUCET} error` as const,
  },
  RECEIVE_FAUCET: {
    action: faucetActions.CLICK_FAUCET,
    category: analyticsCategories.FAUCET,
    label: faucetActions.CLICK_FAUCET,
  },
} as const);

export interface FaucetEventParams {
  address?: MaybeHexString;
  amount?: number;
}

/**
 *
 *    _____                           _
 *  / ____|                         | |
 * | |  __  ___ _ __   ___ _ __ __ _| |
 * | | |_ |/ _ \ '_ \ / _ \ '__/ _` | |
 * | |__| |  __/ | | |  __/ | | (_| | |
 *  \_____|\___|_| |_|\___|_|  \__,_|_|
 *
 * General Analytics Events
 */

export interface GeneralEventParams {
  error?: string;
  network?: string;
  txnHash?: string;
}

export type CombinedEventParams = Omit<
AccountEventParams &
CoinEventParams &
CollectibleEventParams &
FaucetEventParams &
GeneralEventParams
, 'omit'>;

export const analyticsEvent = Object.freeze({
  ...accountEvents,
  ...collectiblesEvents,
  ...coinEvents,
  ...faucetEvents,
} as const);

export type AnalyticsEventType = typeof analyticsEvent;
export type AnalyticsEventTypeKeys = keyof typeof analyticsEvent;
export type AnalyticsEventTypeValues = typeof analyticsEvent[AnalyticsEventTypeKeys];
