// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import Browser from 'core/utils/browser';
import { PromptInfo, PermissionType, PromptMessage } from 'core/types/dappTypes';

const PERMISSIONS_STORAGE_KEY = 'aptosWalletPermissions';
const PROMPT_HEIGHT = 600;
const PROMPT_WIDTH = 375;

export default class Permissions {
  static async getCurrentTab(): Promise<chrome.tabs.Tab> {
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
    return tabs[0];
  }

  static isPromptActive(): Promise<boolean> {
    const { id } = chrome.runtime;
    return new Promise((resolve) => {
      chrome.tabs.query({}, (tabs) => {
        const foundTab = tabs.find((tab) => tab.url?.includes(id));
        resolve(foundTab !== undefined);
      });
    });
  }

  static async promptUser(permission: string): Promise<boolean> {
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

  public static async requestPermissions(
    permission: string,
    domain: string,
    address: string,
  ): Promise<boolean> {
    switch (permission) {
      case PermissionType.CONNECT:
        if (await this.isDomainAllowed(domain, address)) {
          return true;
        }
        if (await this.promptUser(permission)) {
          await this.addDomain(domain, address);
          return true;
        }
        return false;
      case PermissionType.SIGN_AND_SUBMIT_TRANSACTION:
      case PermissionType.SIGN_TRANSACTION:
      case PermissionType.SIGN_MESSAGE:
        if (!await this.isDomainAllowed(domain, address)) {
          return false;
        }
        return this.promptUser(permission);
      default:
        return false;
    }
  }

  static async addDomain(domain: string, address: string): Promise<void> {
    const domains = await this.getDomains(address);
    domains.add(domain);
    return this.saveDomains(domains, address);
  }

  public static async removeDomain(domain: string, address: string): Promise<void> {
    const domains = await this.getDomains(address);
    domains.delete(domain);
    return this.saveDomains(domains, address);
  }

  public static async isDomainAllowed(domain: string, address: string): Promise<boolean> {
    const domains = await this.getDomains(address);
    return domains.has(domain);
  }

  static async getAllDomains(): Promise<{ [address: string]: string[] }> {
    return new Promise((resolve) => {
      Browser.persistentStorage()?.get([PERMISSIONS_STORAGE_KEY], (result) => {
        if (result && result[PERMISSIONS_STORAGE_KEY]) {
          resolve(result[PERMISSIONS_STORAGE_KEY]);
        }
        resolve({});
      });
    });
  }

  public static async getDomains(address: string): Promise<Set<string>> {
    const allDomains = await this.getAllDomains();
    return new Set(allDomains[address]) ?? new Set();
  }

  static async saveDomains(domains: Set<string>, address: string): Promise<void> {
    const allDomains = await this.getAllDomains();
    allDomains[address] = Array.from(domains);
    return Browser.persistentStorage()!.set({ [PERMISSIONS_STORAGE_KEY]: allDomains });
  }
}
