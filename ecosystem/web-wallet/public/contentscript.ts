// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

function injectScript () {
  try {
    const container = document.head || document.documentElement
    const scriptTag = document.createElement('script')
    scriptTag.src = chrome.runtime.getURL('inpage.js')
    container.insertBefore(scriptTag, container.children[0])
    container.removeChild(scriptTag)
  } catch (error) {
    console.error('Aptos injection failed.', error)
  }
}

injectScript()

// inpage -> contentscript
window.addEventListener('message', function (event) {
  if (event.data.method) {
    // contentscript -> background
    chrome.runtime.sendMessage(event.data, function (response) {
      // contentscript -> inpage
      window.postMessage({ responseMethod: event.data.method, id: event.data.id, response })
    })
  }
})
