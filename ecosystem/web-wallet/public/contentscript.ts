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
