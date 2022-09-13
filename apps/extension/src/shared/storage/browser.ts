// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Storage } from 'webextension-polyfill';
import { StorageChanges } from './shared';

type ExtendedAreaName = chrome.storage.AreaName | 'session';
type ChromeStorageChanges = { [key: string]: chrome.storage.StorageChange };
type ChromeStorageChangeCallback = (
  changes: ChromeStorageChanges,
  areaName: ExtendedAreaName,
) => void;

function getStorageAreaName(storage: Storage.StorageArea): ExtendedAreaName {
  return storage === chrome.storage.local
    ? 'local'
    : 'session';
}

// TODO: remove JSON parse/stringify as they're not required for chrome storage

export default class BrowserStorage<TState> {
  constructor(private storage: Storage.StorageArea) {}

  async get<TKey extends keyof TState>(keys: TKey[]) {
    const serializedState = await this.storage.get(keys);

    const entries = Object.entries(serializedState);
    const mappedEntries = entries.map(([key, serialized]) => [
      key,
      serialized ? JSON.parse(serialized) : undefined,
    ]);

    return Object.fromEntries(mappedEntries) as Pick<TState, TKey>;
  }

  async set(values: Partial<TState>) {
    const serializedValues: Record<string, string> = {};
    const keysToRemove: string[] = [];

    Object.entries(values).forEach(([key, value]) => {
      if (value !== undefined) {
        serializedValues[key] = JSON.stringify(value);
      } else {
        keysToRemove.push(key);
      }
    });

    await Promise.all([
      await this.storage.set(serializedValues),
      await this.storage.remove(keysToRemove),
    ]);
  }

  onChange(callback: (changes: StorageChanges<TState>) => void) {
    const onStorageChange: ChromeStorageChangeCallback = (changes, areaName) => {
      if (getStorageAreaName(this.storage) !== areaName) {
        return;
      }

      const mappedChanges: any = {};
      Object.keys(changes).forEach((key) => {
        const change = changes[key] as any;
        const newValue = change?.newValue !== undefined
          ? JSON.parse(change.newValue)
          : undefined;
        const oldValue = change?.oldValue !== undefined
          ? JSON.parse(change.oldValue)
          : undefined;

        mappedChanges[key] = { newValue, oldValue };
      });

      callback(mappedChanges as StorageChanges<TState>);
    };

    chrome.storage.onChanged.addListener(onStorageChange);
    return () => chrome.storage.onChanged.removeListener(onStorageChange);
  }
}
