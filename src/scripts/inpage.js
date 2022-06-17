// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { MessageMethod } from '../core/types';

class Web3 {
  requestId;

  constructor() {
    this.requestId = 0;
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
            && event.data.id === id) {
          const { response } = event.data;
          this.removeEventListener('message', handler);
          if (response.error) {
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
