// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount } from 'aptos';
import Permissions from './permissions';

export const ProviderEvent = Object.freeze({
  ACCOUNT_CHANGED: 'accountChanged',
} as const);

async function sendToTabs(permissionlessMessage: {}, permissionedMessage: {}) {
  const tabs = await chrome.tabs.query({});
  const allowedDomains = await Permissions.getDomains();
  for (let i: number = 0; i < tabs.length; i += 1) {
    const tab = tabs[i];
    if (tab.id && tab.url) {
      const url = new URL(tab.url);
      const message = (allowedDomains.has(url.hostname)
        ? permissionedMessage
        : permissionlessMessage);
      chrome.tabs.sendMessage(tab.id, message);
    }
  }
}

export async function sendProviderEvent(event: string, account: AptosAccount | undefined) {
  switch (event) {
    case ProviderEvent.ACCOUNT_CHANGED:
      await sendToTabs(
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
    default:
      break;
  }
}
