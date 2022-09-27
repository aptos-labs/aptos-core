// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import fetchAdapter from '@vespaiach/axios-fetch-adapter';
import axios from 'axios';
import { DappError, DappErrorType } from 'core/types/errors';
import {
  DappInfo,
  PermissionResponseError,
  PermissionResponseStatus,
} from 'shared/permissions';
import { isAllowedMethodName, PetraPublicApiImpl } from 'shared/petra';
import { SessionStorage, PersistentStorage } from 'shared/storage';
import { isProxiedRequest, makeProxiedResponse, ProxiedResponse } from 'shared/types';

type SendProxiedResult = (result: ProxiedResponse) => void;

// The fetch adapter is necessary to use axios from a service worker
axios.defaults.adapter = fetchAdapter;

chrome.runtime.onMessage.addListener(async (
  request,
  sender,
  sendResponse: SendProxiedResult,
) => {
  if (request.type === 'popupOpened') {
    // clear all pending alarm in case there's any pending alarm that is inflight
    chrome.alarms.clearAll();

    const { autolockTimer } = await PersistentStorage.get(['autolockTimer']);

    // starts timer to lock wallet by default after 15 mins when wallet opens
    // or by number of minutes that user set in Settings
    // for security compliance
    chrome.alarms.create('autolockTimer', {
      delayInMinutes: autolockTimer ?? 15,
    });
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

  const dappInfo = {
    domain: sender.origin,
    imageURI: sender.tab?.favIconUrl,
    name: sender.tab?.title,
  } as DappInfo;

  const methodBody = PetraPublicApiImpl[request.method] as (...args: any[]) => Promise<any>;
  methodBody(dappInfo, ...request.args)
    .then((result) => {
      sendResponse(makeProxiedResponse(request.id, result));
    })
    .catch((error) => {
      // Unmanaged errors are obfuscated before being sent back to the dapp
      if (error instanceof DappError) {
        sendResponse(makeProxiedResponse(request.id, error));
      } else if (error instanceof PermissionResponseError) {
        const dappError = error.status === PermissionResponseStatus.Rejected
          ? DappErrorType.USER_REJECTION
          : DappErrorType.TIME_OUT;
        sendResponse(makeProxiedResponse(request.id, dappError));
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

    // if autolock timer not yet set when wallet closes, default timer to 15 mins
    // starts timer as soon as user close the wallet and become 'inactive'
    // to satisfy security compliance requirement
    chrome.alarms.create('autolockTimer', {
      delayInMinutes: autolockTimer ?? 15,
    });
  });
});
