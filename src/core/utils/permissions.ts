// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import Browser from 'core/utils/browser';
import { PromptInfo, PermissionType, PromptMessage } from 'core/types';

const PERMISSIONS_STORAGE_KEY = 'aptosWalletPermissions';
const PROMPT_HEIGHT = 600;
const PROMPT_WIDTH = 375;

export default class Permissions {
  static async getCurrentTab(): Promise<chrome.tabs.Tab> {
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
    return tabs[0];
  }

  static async promptUser(permission: string): Promise<boolean> {
    const { favIconUrl, title, url } = await this.getCurrentTab();
    const window = await chrome.windows.getCurrent();
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
    return new Promise((resolve) => {
      chrome.runtime.onMessage.addListener(function handler(request, sender, sendResponse) {
        switch (request) {
          case PromptMessage.LOADED:
            // eslint-disable-next-line no-case-declarations
            const info: PromptInfo = {
              domain: url ? new URL(url).hostname : undefined,
              imageURI: favIconUrl,
              promptType: permission,
              title,
            };
            sendResponse(info);
            break;
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

  public static async requestPermissions(permission: string, domain: string): Promise<boolean> {
    switch (permission) {
      case PermissionType.CONNECT:
        if (await this.isDomainAllowed(domain)) {
          return true;
        }
        if (await this.promptUser(permission)) {
          await this.addDomain(domain);
          return true;
        }
        return false;
      case PermissionType.SIGN_AND_SUBMIT_TRANSACTION:
      case PermissionType.SIGN_TRANSACTION:
        if (!await this.isDomainAllowed(domain)) {
          return false;
        }
        return this.promptUser(permission);
      default:
        return false;
    }
  }

  static async addDomain(domain: string): Promise<void> {
    const domains = await this.getDomains();
    domains.add(domain);
    return this.saveDomains(domains);
  }

  public static async removeDomain(domain: string): Promise<void> {
    const domains = await this.getDomains();
    domains.delete(domain);
    return this.saveDomains(domains);
  }

  public static async isDomainAllowed(domain: string): Promise<boolean> {
    const domains = await this.getDomains();
    return domains.has(domain);
  }

  static async getDomains(): Promise<Set<string>> {
    const result = await Browser.storage()?.get([PERMISSIONS_STORAGE_KEY]);
    if (result && result[PERMISSIONS_STORAGE_KEY]) {
      return new Set(result[PERMISSIONS_STORAGE_KEY]);
    }
    return new Set();
  }

  static saveDomains(domains: Set<string>): Promise<void> {
    return Browser.storage()!.set({ [PERMISSIONS_STORAGE_KEY]: Array.from(domains) });
  }
}
