// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PetraPublicApiImpl, isAllowedMethodName } from 'shared/petra';
import { isProxiedRequest, makeProxiedResponse, ProxiedResponse } from 'shared/types';
import { DappError, DappErrorType } from 'core/types/errors';

type SendProxiedResult = (result: ProxiedResponse) => void;

chrome.runtime.onMessage.addListener((
  request,
  sender,
  sendResponse: SendProxiedResult,
) => {
  if (!isProxiedRequest(request)) {
    return false;
  }

  // This is checked in content script already, but double-checking to be sure
  if (!isAllowedMethodName(request.method)) {
    makeProxiedResponse(request.id, DappErrorType.UNSUPPORTED);
    sendResponse(makeProxiedResponse(request.id, DappErrorType.UNSUPPORTED));
    return false;
  }

  const methodBody = PetraPublicApiImpl[request.method] as (...args: any[]) => Promise<any>;
  methodBody(...request.args)
    .then((result) => {
      sendResponse(makeProxiedResponse(request.id, result));
    })
    .catch((error) => {
      // Unmanaged errors are obfuscated before being sent back to the dapp
      if (error instanceof DappError) {
        sendResponse(makeProxiedResponse(request.id, error));
      } else {
        sendResponse(makeProxiedResponse(request.id, DappErrorType.INTERNAL_ERROR));
      }

      // Error is rethrown so that it can be inspected
      throw error;
    });

  // Return true to indicate the response is asynchronous
  return true;
});
