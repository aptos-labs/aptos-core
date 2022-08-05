// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { HexString, TxnBuilderTypes, BCS } from 'aptos';
import { MessageMethod } from '../core/types/dappTypes';
import { createRawTransaction, getSigningMessage } from '../core/utils/transaction';

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

  chainId() {
    return this.message(MessageMethod.GET_CHAIN_ID, {});
  }

  sequenceNumber() {
    return this.message(MessageMethod.GET_SEQUENCE_NUMBER, {});
  }

  signMessage(message) {
    return this.message(MessageMethod.SIGN_MESSAGE, { message });
  }

  async signAndSubmitTransaction(transaction) {
    const signedTransaction = await this.signTransaction(transaction);
    return this.message(
      MessageMethod.SUBMIT_TRANSACTION,
      { signedTransaction: HexString.fromUint8Array(signedTransaction).hex() },
    );
  }

  async signTransaction(transaction) {
    const [{ chainId }, { sequenceNumber }, { address, publicKey }] = await Promise.all(
      [this.chainId(), this.sequenceNumber(), this.account()],
    );

    const rawTransaction = createRawTransaction(transaction, {
      chainId,
      sender: address,
      sequenceNumber,
    });

    const { signature: sigHexStr } = await this.message(
      MessageMethod.SIGN_TRANSACTION,
      { signingMessage: getSigningMessage(rawTransaction) },
    );
    const signature = new TxnBuilderTypes.Ed25519Signature(
      new HexString(sigHexStr).toUint8Array(),
    );

    const authenticator = new TxnBuilderTypes.TransactionAuthenticatorEd25519(
      new TxnBuilderTypes.Ed25519PublicKey(new HexString(publicKey).toUint8Array()),
      signature,
    );

    return BCS.bcsToBytes(new TxnBuilderTypes.SignedTransaction(rawTransaction, authenticator));
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
