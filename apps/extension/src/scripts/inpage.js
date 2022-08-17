// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { MessageMethod } from '../core/types/dappTypes';

class Web3 {
  requestId;

  eventListenerMap;

  constructor() {
    this.requestId = 0;
    this.eventListenerMap = {};

    // init the event listener helper
    window.addEventListener('message', (request) => {
      const { data } = request;
      if (data && this.eventListenerMap[data.event]) {
        this.eventListenerMap[data.event](data.params);
      }
    });
  }

  on(event, callback) {
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

  signMessage(message) {
    return this.message(MessageMethod.SIGN_MESSAGE, { message });
  }

  signAndSubmitTransaction(transaction) {
    return this.message(MessageMethod.SIGN_AND_SUBMIT_TRANSACTION, { transaction });
  }

  signTransaction(transaction) {
    return this.message(MessageMethod.SIGN_TRANSACTION, { transaction });
  }

  message(method, args) {
    this.requestId += 1;
    const id = this.requestId;
    return new Promise((resolve, reject) => {
      window.postMessage({ args, id, method });
      window.addEventListener('message', function handler(event) {
        if (event.data.responseMethod === method
            && event.data.id === id
            && (event.data.response !== undefined
              || event.data.response !== null)) {
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

window.aptos = new Web3();
