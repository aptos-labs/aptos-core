// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount } from 'aptos';
import Browser from './browser';
import { getLocalStorageNodeNetworkUrl, nodeUrlReverseMap } from './network';
import Permissions from './permissions';

export const ProviderEvent = Object.freeze({
  ACCOUNT_CHANGED: 'accountChanged',
  NETWORK_CHANGED: 'networkChanged',
} as const);

async function sendToTabs(
  address: string | undefined,
  permissionlessMessage: {},
  permissionedMessage: {},
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

export async function sendProviderEvent(event: string, account: AptosAccount | undefined) {
  switch (event) {
    case ProviderEvent.ACCOUNT_CHANGED:
      await sendToTabs(
        account?.address().hex(),
        { event, params: {} },
        {
          event,
          params: {
            address: account?.address().hex(),
            publicKey: account?.pubKey().hex(),
          },
        },
      );
      break;
    case ProviderEvent.NETWORK_CHANGED: {
      const network = nodeUrlReverseMap[getLocalStorageNodeNetworkUrl()];
      await sendToTabs(
        account?.address().hex(),
        { event, params: network },
        { event, params: network },
      );
      break;
    }
    default:
      break;
  }
}
