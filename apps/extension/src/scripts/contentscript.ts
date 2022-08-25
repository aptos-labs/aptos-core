// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { isProxiedRequest, makeProxiedResponse, ProxiedResponse } from 'shared/types';
import { isAllowedMethodName } from 'shared/petra';
import { DappErrorType } from 'core/types/errors';

function injectScript() {
  try {
    const container = document.head || document.documentElement;
    const scriptTag = document.createElement('script');
    scriptTag.src = chrome.runtime.getURL('static/js/inpage.js');
    container.insertBefore(scriptTag, container.children[0]);
    container.removeChild(scriptTag);
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error('Aptos injection failed.', error);
  }
}

injectScript();

// inpage -> contentscript
window.addEventListener('message', (event) => {
  if (!isProxiedRequest(event.data)) {
    return;
  }

  const { data: request } = event;

  // This is also re-checked in service worker, just in case
  if (!isAllowedMethodName(request.method)) {
    window.postMessage(makeProxiedResponse(request.id, DappErrorType.UNSUPPORTED));
    return;
  }

  // contentscript -> background
  chrome.runtime.sendMessage(request, (response: ProxiedResponse) => {
    // contentscript -> inpage
    window.postMessage(response);
  });
});

// Send extension messages to window for event listening
chrome.runtime.onMessage.addListener((message) => {
  window.postMessage(message);
});
