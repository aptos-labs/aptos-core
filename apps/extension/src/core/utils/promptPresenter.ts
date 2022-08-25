// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PromptInfo, PromptMessage, PromptType } from 'core/types/dappTypes';

const PROMPT_HEIGHT = 600;
const PROMPT_WIDTH = 375;

export default class PromptPresenter {
  static async getCurrentTab(): Promise<chrome.tabs.Tab> {
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
    return tabs[0];
  }

  static async isPromptActive() {
    const { id: extensionId } = chrome.runtime;
    const tabs = await chrome.tabs.query({});
    const foundTab = tabs.find((tab) => {
      const url = tab.url ? new URL(tab.url) : undefined;
      return url?.hostname === extensionId && url?.pathname === '/prompt.html';
    });
    return foundTab !== undefined;
  }

  static async promptUser(promptType: PromptType): Promise<boolean> {
    const isPromptActive = await this.isPromptActive();
    if (isPromptActive) {
      return false;
    }
    const { favIconUrl, title, url } = await this.getCurrentTab();
    chrome.windows.getCurrent(async (window) => {
      const left = (window.left ?? 0) + (window.width ?? 0) - PROMPT_WIDTH;
      const { top } = window;
      await chrome.windows.create({
        height: PROMPT_HEIGHT,
        left,
        top,
        type: 'popup',
        url: 'prompt.html',
        width: PROMPT_WIDTH,
      });
    });

    return new Promise((resolve) => {
      chrome.runtime.onMessage.addListener(function handler(request, sender, sendResponse) {
        switch (request) {
          case PromptMessage.LOADED: {
            const info: PromptInfo = {
              domain: url ? new URL(url).hostname : undefined,
              imageURI: favIconUrl,
              promptType,
              title,
            };
            sendResponse(info);

            // if it's a  warning remove the listener after load
            switch (promptType.kind) {
              case 'permission':
                break;
              case 'warning':
              default:
                chrome.runtime.onMessage.removeListener(handler);
                resolve(true);
            }
            break;
          }
          case PromptMessage.APPROVED:
            resolve(true);
            chrome.runtime.onMessage.removeListener(handler);
            sendResponse();
            break;
          case PromptMessage.REJECTED:
            resolve(false);
            chrome.runtime.onMessage.removeListener(handler);
            sendResponse();
            break;
          default:
            break;
        }
      });
    });
  }
}
