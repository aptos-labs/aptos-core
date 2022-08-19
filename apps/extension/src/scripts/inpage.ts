// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { MessageMethod } from 'core/types/dappTypes';
import { ProviderEvent, ProviderMessage } from 'core/utils/providerEvents';
import { TransactionPayload } from 'aptos/dist/generated';

type OnProviderMessageCallback = (params: any) => void;

class Web3 {
  requestId;

  eventListenerMap: Record<string, OnProviderMessageCallback>;

  constructor() {
    this.requestId = 0;
    this.eventListenerMap = {};

    // init the event listener helper
    window.addEventListener('message', (request: MessageEvent<ProviderMessage>) => {
      const { data } = request;
      if (data && this.eventListenerMap[data.event]) {
        this.eventListenerMap[data.event](data.params);
      }
    });
  }

  on(event: ProviderEvent, callback: OnProviderMessageCallback) {
    this.eventListenerMap[event] = callback;
  }

  connect() {
    return this.message(MessageMethod.CONNECT, {});
  }

  disconnect() {
    return this.message(MessageMethod.DISCONNECT, {});
  }

  isConnected() {
    return this.message(MessageMethod.IS_CONNECTED, {});
  }

  account() {
    return this.message(MessageMethod.GET_ACCOUNT_ADDRESS, {});
  }

  network() {
    return this.message(MessageMethod.GET_NETWORK, {});
  }

  signMessage(message: string) {
    return this.message(MessageMethod.SIGN_MESSAGE, { message });
  }

  async signAndSubmitTransaction(transaction: TransactionPayload) {
    return this.message(MessageMethod.SIGN_AND_SUBMIT_TRANSACTION, { transaction });
  }

  async signTransaction(transaction: TransactionPayload) {
    return this.message(MessageMethod.SIGN_TRANSACTION, { transaction });
  }

  message(method: MessageMethod, args: any) {
    this.requestId += 1;
    const id = this.requestId;
    return new Promise<any>((resolve, reject) => {
      window.postMessage({ args, id, method });
      window.addEventListener('message', function handler(event) {
        if (event.data.responseMethod === method
            && event.data.id === id
            && (event.data.response !== undefined || true)) {
          const { response } = event.data;
          this.removeEventListener('message', handler);
          if (response && response.error) {
            reject(response.error ?? 'Error');
          } else {
            resolve(response);
          }
        }
      });
    });
  }
}

(window as any).aptos = new Web3();
