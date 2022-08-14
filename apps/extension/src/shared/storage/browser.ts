// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Storage } from 'webextension-polyfill';

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
}
