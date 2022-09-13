// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Storage } from 'webextension-polyfill';
import { StorageChanges } from './shared';

const windowStorageSimulatedAccessTimeMs = 50;

// Utility function for simulating async nature of browser storage
function sleep(milliseconds: number) {
  return new Promise((resolve) => {
    setTimeout(resolve, milliseconds);
  });
}

export default class WindowStorage<TState> {
  constructor(private storage: Storage) {}

  async get<TKey extends keyof TState>(keys: TKey[]) {
    await sleep(windowStorageSimulatedAccessTimeMs);

    const values = {} as Pick<TState, TKey>;
    keys.forEach((key) => {
      const serialized = this.storage.getItem(key as string);
      values[key] = serialized ? JSON.parse(serialized) : undefined;
    });

    return values;
  }

  async set(values: Partial<TState>) {
    await sleep(windowStorageSimulatedAccessTimeMs);

    Object.entries(values).forEach(([key, value]) => {
      if (value !== undefined) {
        const serialized = JSON.stringify(value);
        this.storage.setItem(key, serialized);
      } else {
        this.storage.removeItem(key);
      }
    });
  }

  onChange(callback: (changes: StorageChanges<TState>) => void) {
    const onStorageChange = (event: StorageEvent) => {
      if (event.storageArea !== this.storage && event.key !== null) {
        return;
      }

      const key = event.key as any;
      const newValue = event.newValue !== null
        ? JSON.parse(event.newValue) as any
        : undefined;
      const oldValue = event.oldValue !== null
        ? JSON.parse(event.oldValue) as any
        : undefined;

      callback({ [key]: { newValue, oldValue } });
    };

    window.addEventListener('storage', onStorageChange);
    return () => window.removeEventListener('storage', onStorageChange);
  }
}
