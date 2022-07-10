// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { MaybeHexString } from 'aptos';
import { AptosNetwork } from '../network';

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
  LOGIN_WITH_PRIVATE_KEY: 'Login with private key',
  SIGN_OUT: 'Sign out',
} as const);

const accountLabels = Object.freeze({
  CREATE_ACCOUNT: 'Create account',
  LOGIN_WITH_PRIVATE_KEY: 'Login',
  SIGN_OUT: 'Sign out',
} as const);

export const loginEvents = eventSchemaTypeCheck({
  ERROR_LOGIN_WITH_PRIVATE_KEY: {
    action: `${accountActions.LOGIN_WITH_PRIVATE_KEY} - invalid private key`,
    category: analyticsCategories.ACCOUNT,
    label: `${accountLabels.LOGIN_WITH_PRIVATE_KEY} error`,
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
    action: `${accountActions.CREATE_ACCOUNT} - unable to create account`,
    category: analyticsCategories.ACCOUNT,
    label: `${accountLabels.CREATE_ACCOUNT} error`,
  },
} as const);

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
    action: `${coinActions.TRANSFER_COIN} - unable to transfer coin`,
    category: analyticsCategories.COIN,
    label: `${coinLabels.TRANSFER_APTOS_COIN} error`,
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
    action: `${collectiblesActions.CLAIM_TOKEN} - unable to claim token`,
    category: analyticsCategories.COLLECTIBLES,
    label: `${collectiblesLabels.CLAIM_TOKEN} error`,
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
  RECEIVE_FAUCET: {
    action: faucetActions.CLICK_FAUCET,
    category: analyticsCategories.FAUCET,
    label: undefined,
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
  network?: AptosNetwork;
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
