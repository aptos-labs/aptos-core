// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PetraPublicApiImpl, isAllowedMethodName } from 'shared/petra';
import { isProxiedRequest, makeProxiedResponse, ProxiedResponse } from 'shared/types';
import { DappError, DappErrorType } from 'core/types/errors';
import { SessionStorage, PersistentStorage } from 'shared/storage';

type SendProxiedResult = (result: ProxiedResponse) => void;

chrome.runtime.onMessage.addListener((
  request,
  sender,
  sendResponse: SendProxiedResult,
) => {
  // clear all pending alarm to prevent wallet being locked while being used
  if (request.type === 'popupOpened') {
    chrome.alarms.clearAll();
  }

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
        // Internal unexpected error, rethrow so we can inspect
        throw error;
      }
    });

  // Return true to indicate the response is asynchronous
  return true;
});

// lock account as soon as alarm timer elapsed
chrome.alarms.onAlarm.addListener(async () => {
  await SessionStorage.set({
    accounts: undefined,
    encryptionKey: undefined,
  });
});

chrome.runtime.onConnect.addListener((port) => {
  port.onDisconnect.addListener(async () => {
    const { autolockTimer } = await PersistentStorage.get(['autolockTimer']);

    // if autolock timer not yet set, exit early
    if (!autolockTimer) return;

    // starts timer as soon as user close the wallet and become 'inactive'
    chrome.alarms.create('autolockTimer', {
      delayInMinutes: autolockTimer,
    });
  });
});
