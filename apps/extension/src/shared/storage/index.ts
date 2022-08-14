// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PersistentState, SessionState } from 'shared/types';
import BrowserStorage from './browser';
import WindowStorage from './window';

const hasBrowserStorage = Boolean(chrome.storage);

export const PersistentStorage = hasBrowserStorage
  ? new BrowserStorage<PersistentState>(chrome.storage.local)
  : new WindowStorage<PersistentState>(window.localStorage);

export const SessionStorage = hasBrowserStorage
  ? new BrowserStorage<SessionState>((chrome.storage as any).session)
  : new WindowStorage<SessionState>(window.sessionStorage);
