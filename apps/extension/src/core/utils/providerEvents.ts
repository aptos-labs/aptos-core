// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { getBackgroundCurrentPublicAccount, getBackgroundNetwork } from './account';
import Browser from './browser';
import Permissions from './permissions';

export enum ProviderEvent {
  ACCOUNT_CHANGED = 'accountChanged',
  NETWORK_CHANGED = 'networkChanged',
}

export interface ProviderMessage {
  event: ProviderEvent,
  params?: any,
}

async function sendToTabs(
  address: string | undefined,
  permissionlessMessage: ProviderMessage,
  permissionedMessage: ProviderMessage,
) {
  const tabs = await Browser.tabs()?.query({});
  if (tabs) {
    const allowedDomains = address ? await Permissions.getDomains(address) : new Set();
    for (let i: number = 0; i < tabs.length; i += 1) {
      const tab = tabs[i];
      if (tab.id && tab.url) {
        const url = new URL(tab.url);
        const message = (allowedDomains.has(url.hostname)
          ? permissionedMessage
          : permissionlessMessage);
        Browser.tabs()?.sendMessage(tab.id, message);
      }
    }
  }
}

export async function sendProviderEvent(event: ProviderEvent) {
  const publicAccount = await getBackgroundCurrentPublicAccount();
  switch (event) {
    case ProviderEvent.ACCOUNT_CHANGED:
      await sendToTabs(
        publicAccount?.address,
        { event, params: {} },
        {
          event,
          params: {
            address: publicAccount?.address,
            publicKey: publicAccount?.publicKey,
          },
        },
      );
      break;
    case ProviderEvent.NETWORK_CHANGED: {
      const network = (await getBackgroundNetwork()).name;
      await sendToTabs(
        publicAccount?.address,
        { event, params: network },
        { event, params: network },
      );
      break;
    }
    default:
      break;
  }
}
