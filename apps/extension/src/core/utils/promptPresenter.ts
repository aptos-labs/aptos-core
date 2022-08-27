// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PromptInfo, PromptMessage, PromptType } from 'core/types/dappTypes';
import { DappErrorType } from 'core/types/errors';

const PROMPT_HEIGHT = 600;
const PROMPT_WIDTH = 375;

export default class PromptPresenter {
  static async getCurrentTab(): Promise<chrome.tabs.Tab> {
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
    return tabs[0];
  }

  private static async currentPrompt() {
    const { id: extensionId } = chrome.runtime;
    const tabs = await chrome.tabs.query({});
    const foundTab = tabs.find((tab) => {
      const url = tab.url ? new URL(tab.url) : undefined;
      return url?.hostname === extensionId && url?.pathname === '/prompt.html';
    });
    return foundTab;
  }

  private static async listenForPrompts(info: PromptInfo): Promise<boolean> {
    return new Promise((resolve, reject) => {
      chrome.runtime.onMessage.addListener(function handler(request, sender, sendResponse) {
        switch (request) {
          case PromptMessage.LOADED: {
            sendResponse(info);
            // if it's a  warning remove the listener after load
            switch (info.promptType.kind) {
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
          case PromptMessage.TIME_OUT:
            reject(DappErrorType.TIME_OUT);
            chrome.runtime.onMessage.removeListener(handler);
            sendResponse();
            break;
          default:
            break;
        }
      });
    });
  }

  // Send a message to a tab and await the promised response
  private static async sendAsyncMessage(id: number, object: any): Promise<void> {
    return new Promise((resolve) => {
      chrome.tabs.sendMessage(
        id,
        object,
        async (response: Promise<any>) => {
          await response;
          resolve();
        },
      );
    });
  }

  /**
    * Prompts user with new window
    * @throws {DappErrorType.TIME_OUT} if the user doesn't respond in time
  */
  static async promptUser(promptType: PromptType): Promise<boolean> {
    const { favIconUrl, title, url } = await this.getCurrentTab();
    const info: PromptInfo = {
      domain: url ? new URL(url).hostname : undefined,
      imageURI: favIconUrl,
      promptType,
      title,
    };
    const currentPrompt = await this.currentPrompt();
    if (currentPrompt) {
      // focus old prompt
      chrome.windows.update(currentPrompt.windowId!, { focused: true }, () => {});
      // send message to time out old prompt and render new info
      await this.sendAsyncMessage(currentPrompt.id!, { promptInfo: info });
      return this.listenForPrompts(info);
    }
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
    return this.listenForPrompts(info);
  }
}
