// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PublicAccount } from 'shared/types';
import Browser from './browser';
import Permissions from './permissions';

export enum ProviderEvent {
  ACCOUNT_CHANGED = 'accountChanged',
  NETWORK_CHANGED = 'networkChanged',
}

export interface AccountChangeParams {
  address: string | undefined,
  publicKey: string | undefined,
}

export interface NetworkChangeParams {
  networkName: string;
}

export type PetraEventParams = NetworkChangeParams | AccountChangeParams;

export interface ProviderMessage {
  event: ProviderEvent,
  params: PetraEventParams,
}

async function sendToTabs(
  address: string | undefined,
  event: ProviderEvent,
  permissionlessParams?: PetraEventParams,
  permissionedParams?: PetraEventParams,
) {
  const tabs = await Browser.tabs()?.query({});
  if (tabs) {
    const allowedDomains = address ? await Permissions.getDomains(address) : new Set();
    tabs.forEach((tab) => {
      if (tab.id && tab.url) {
        const url = new URL(tab.url);
        const isAllowed = allowedDomains.has(url.hostname);
        const params = isAllowed ? permissionedParams : permissionlessParams;
        Browser.tabs()?.sendMessage(tab.id, { event, params });
      }
    });
  }
}

export async function triggerAccountChange(newPublicAccount: PublicAccount | undefined) {
  await sendToTabs(
    newPublicAccount?.address,
    ProviderEvent.ACCOUNT_CHANGED,
    { address: undefined, publicKey: undefined },
    {
      address: newPublicAccount?.address,
      publicKey: newPublicAccount?.publicKey,
    },
  );
}

export async function triggerNetworkChange(
  currAddress: string | undefined,
  params: NetworkChangeParams,
) {
  await sendToTabs(
    currAddress,
    ProviderEvent.NETWORK_CHANGED,
    params,
    params,
  );
}
