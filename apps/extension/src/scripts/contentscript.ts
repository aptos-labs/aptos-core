// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

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
  if (event.data.method) {
    // contentscript -> background
    chrome.runtime.sendMessage(event.data, (response) => {
      // contentscript -> inpage
      window.postMessage({ id: event.data.id, response, responseMethod: event.data.method });
    });
  }
});

// Send extension messages to window for event listening
chrome.runtime.onMessage.addListener((message) => {
  window.postMessage(message);
});

export {};
