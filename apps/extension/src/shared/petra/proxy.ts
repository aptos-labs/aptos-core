// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { isProxiedResponse, makeProxiedRequest } from 'shared/types';
import { PetraEventListener, PetraPublicApi, PetraPublicApiMethod } from 'shared/petra';
import { DappError } from 'core/types/errors';

function isDappError(error: DappError): error is DappError {
  return error.code !== undefined
    && error.name !== undefined
    && error.message !== undefined;
}

/**
 * Proxy for the Petra public API, that will forward method calls to the extension service worker
 * through the extension content script
 */
export class PetraPublicApiProxy extends PetraEventListener implements PetraPublicApi {
  // TODO: consider making this a random uuid instead
  private requestId: number = 0;

  private proxiedMethod(method: PetraPublicApiMethod) {
    // TODO: enforce type to args and result
    return (...args: any[]) => this.forwardCall(method, args);
  }

  private async forwardCall(method: string, args: any[]) {
    const currRequestId = this.requestId;
    this.requestId += 1;

    return new Promise<any>((resolve, reject) => {
      window.postMessage(makeProxiedRequest(currRequestId, method, args));
      window.addEventListener('message', function handler({ data: response }: MessageEvent<any>) {
        // Just ignore response from non-matching requests
        if (!isProxiedResponse(response) || response.id !== currRequestId) {
          return;
        }
        this.removeEventListener('message', handler);

        if (response.error !== undefined) {
          // Reconstruct dapp error so that it can be caught by the consumer
          if (isDappError(response.error)) {
            const { code, message, name } = response.error;
            reject(new DappError(code, name, message));
          } else {
            reject(response.error);
          }
        } else {
          resolve(response.result);
        }
      });
    });
  }

  account = this.proxiedMethod('account');

  connect = this.proxiedMethod('connect');

  disconnect = this.proxiedMethod('disconnect');

  isConnected = this.proxiedMethod('isConnected');

  network = this.proxiedMethod('network');

  signAndSubmitTransaction = this.proxiedMethod('signAndSubmitTransaction');

  signMessage = this.proxiedMethod('signMessage');

  signTransaction = this.proxiedMethod('signTransaction');
}

export default PetraPublicApiProxy;
