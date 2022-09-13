// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PersistentStorage } from 'shared/storage';
import {
  isPermissionResponse,
  DappInfo,
  PermissionRequest,
  PermissionResponse,
  Permission, PermissionResponseStatus,
} from '../types';
import {
  handlePermissionResponse,
  PROMPT_SIZE,
  PROMPT_PATHNAME,
} from './shared';

async function getCurrentPrompt() {
  // TODO: put proper filter in query instead of searching every tab
  const { id: extensionId } = chrome.runtime;
  const tabs = await chrome.tabs.query({});
  return tabs.find((tab) => {
    const url = tab.url ? new URL(tab.url) : undefined;
    return url?.hostname === extensionId && url?.pathname === `/${PROMPT_PATHNAME}`;
  });
}

async function openPrompt() {
  const { height, width } = PROMPT_SIZE;
  const window = await chrome.windows.getCurrent();
  const left = (window.left ?? 0) + (window.width ?? 0) - width;
  const { top } = window;

  // Note: on Kiwi browser creating a window with a default tab doesn't work
  //  as the window.tabs property is undefined. As a workaround, we create a popup window first,
  //  then we create and assign the prompt tab to it.

  const promptWindow = await chrome.windows.create({
    height,
    left,
    top,
    type: 'popup',
    width,
  });

  if (promptWindow.id === undefined) {
    throw new Error('Prompt window was created without an id');
  }

  const promptTab = await chrome.tabs.create({
    active: true,
    url: PROMPT_PATHNAME,
    windowId: promptWindow.id,
  });

  if (promptTab.id === undefined) {
    throw new Error('Prompt tab was created without an id');
  }

  return promptTab;
}

async function waitForResponse(promptTabId: number, requestId: number) {
  return new Promise<PermissionResponse>((resolve) => {
    const listeners = {
      onMessage: (response: any, sender: chrome.runtime.MessageSender) => {
        const isFromPrompt = sender.tab?.id === promptTabId;
        if (isFromPrompt
          && isPermissionResponse(response)
          && response.id === requestId) {
          chrome.runtime.onMessage.removeListener(listeners.onMessage);
          chrome.tabs.onRemoved.removeListener(listeners.onTabRemoved);
          resolve(response);
        }
      },
      onTabRemoved: (tabId: number) => {
        if (tabId === promptTabId) {
          chrome.runtime.onMessage.removeListener(listeners.onMessage);
          chrome.tabs.onRemoved.removeListener(listeners.onTabRemoved);
          resolve({ id: requestId, status: PermissionResponseStatus.Rejected });
        }
      },
    };

    chrome.runtime.onMessage.addListener(listeners.onMessage);
    chrome.tabs.onRemoved.addListener(listeners.onTabRemoved);
  });
}

export async function requestPermission(
  dappInfo: DappInfo,
  permission: Permission,
) {
  const permissionRequest = {
    dappInfo,
    id: Date.now(),
    permission,
  } as PermissionRequest;
  await PersistentStorage.set({ permissionRequest });

  const promptTab = await getCurrentPrompt() ?? await openPrompt();
  if (promptTab.id === undefined || promptTab.id === chrome.tabs.TAB_ID_NONE) {
    throw new Error('Invalid prompt tab id');
  }
  await chrome.windows.update(promptTab.windowId, { focused: true });
  const response = await waitForResponse(promptTab.id, permissionRequest.id);
  return handlePermissionResponse(response);
}

export async function sendPermissionResponse(response: PermissionResponse) {
  await chrome.runtime.sendMessage(response);
}
