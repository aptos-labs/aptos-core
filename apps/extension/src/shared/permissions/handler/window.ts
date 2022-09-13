// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PersistentStorage } from 'shared/storage';
import {
  DappInfo,
  isPermissionResponse,
  Permission,
  PermissionRequest,
  PermissionResponse,
  PermissionResponseStatus,
} from '../types';
import {
  handlePermissionResponse,
  PROMPT_PATHNAME,
  PROMPT_SIZE,
} from './shared';

const PROMPT_POLLER_INTERVAL = 500;
let gCurrPromptRef: Window | undefined;

async function openPrompt() {
  const { height, width } = PROMPT_SIZE;
  const params = {
    height,
    left: window.screenLeft + window.outerWidth - width,
    popup: true,
    top: window.screenTop,
    width,
  };

  const strParams = Object.entries(params)
    .map(([key, value]) => `${key}=${JSON.stringify(value)}`)
    .reduce((acc, entry) => `${acc}, ${entry}`);

  const promptWindow = window.open(PROMPT_PATHNAME, 'prompt', strParams);
  if (promptWindow === null) {
    throw new Error("Couldn't open permission request popup");
  }

  gCurrPromptRef = promptWindow;
  return promptWindow;
}

function getCurrPrompt() {
  return gCurrPromptRef !== undefined && !gCurrPromptRef.closed
    ? gCurrPromptRef
    : undefined;
}

async function waitForResponse(promptWindow: Window, requestId: number) {
  return new Promise<PermissionResponse>((resolve) => {
    const listeners = {
      onMessage: (message: MessageEvent<any>) => {
        if (message.source === promptWindow
          && isPermissionResponse(message.data)
          && (message.data.id === requestId)) {
          window.removeEventListener('message', listeners.onMessage);
          clearTimeout(listeners.promptPollerId);
          resolve(message.data);
        }
      },
      promptPollerId: setInterval(() => {
        const isPromptClosed = gCurrPromptRef?.closed !== false;
        if (isPromptClosed) {
          window.removeEventListener('message', listeners.onMessage);
          clearTimeout(listeners.promptPollerId);
          resolve({ id: requestId, status: PermissionResponseStatus.Rejected });
        }
      }, PROMPT_POLLER_INTERVAL),
    };

    window.addEventListener('message', listeners.onMessage);
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
  const promptWindow = getCurrPrompt() ?? await openPrompt();
  promptWindow.focus();
  const response = await waitForResponse(promptWindow, permissionRequest.id);
  return handlePermissionResponse(response);
}

/**
 * Send a response to the main window
 * @param response
 */
export async function sendPermissionResponse(response: PermissionResponse) {
  const parentWindow = window.opener as Window;
  parentWindow.postMessage(response);
}
