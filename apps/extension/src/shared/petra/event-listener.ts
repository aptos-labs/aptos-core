// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AccountChangeParams,
  NetworkChangeParams,
  ProviderEvent,
  ProviderMessage,
} from 'core/utils/providerEvents';

type EventCallback<T = any> = (args: T) => void;

function isProviderMessage(message: ProviderMessage): message is ProviderMessage {
  return Object.values(ProviderEvent).includes(message?.event);
}

export class PetraEventListener {
  // private eventListenerMap = {} as Record<ProviderEvent, EventCallback>;
  private readonly eventListenerMap: Record<string, EventCallback>;

  constructor() {
    this.eventListenerMap = {};
    window.addEventListener('message', (msg: MessageEvent<any>) => {
      if (!isProviderMessage(msg.data)) {
        return;
      }

      const { event, params } = msg.data;
      if (this.eventListenerMap[event]) {
        const callback = this.eventListenerMap[event];
        callback(params);
      }
    });
  }

  onAccountChange(callback?: EventCallback<AccountChangeParams>) {
    this.on(ProviderEvent.ACCOUNT_CHANGED, callback);
  }

  onNetworkChange(callback?: EventCallback<NetworkChangeParams>) {
    this.on(ProviderEvent.NETWORK_CHANGED, callback);
  }

  on(event: ProviderEvent, callback?: EventCallback) {
    if (callback !== undefined) {
      this.eventListenerMap[event] = callback;
    } else {
      delete this.eventListenerMap[event];
    }
  }
}

export default PetraEventListener;
