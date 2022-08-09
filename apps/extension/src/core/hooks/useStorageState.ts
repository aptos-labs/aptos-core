// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Storage } from 'webextension-polyfill';
import { useEffect, useState } from 'react';

// Utility function for simulating async nature of browser storage
function sleep(milliseconds: number) {
  return new Promise((resolve) => {
    setTimeout(resolve, milliseconds);
  });
}

type AsyncSetter<T> = (newValue?: T) => Promise<void>;
type UseStorageStateResult<T> = [T | undefined, AsyncSetter<T>, boolean];

// region Window storage state

const windowStorageSimulatedAccessTimeMs = 10;

export function useWindowStorageState<T>(
  storage: Storage,
  key: string,
  defaultValue?: T,
) : UseStorageStateResult<T> {
  const [value, setValue] = useState<T>();
  const [isReady, setIsReady] = useState<boolean>(false);

  useEffect(() => {
    sleep(windowStorageSimulatedAccessTimeMs).then(() => {
      const serialized = storage.getItem(key);
      const initialValue = (serialized && JSON.parse(serialized)) || defaultValue;
      setValue(initialValue);
      setIsReady(true);
    });
  }, [key, defaultValue, setValue, setIsReady, storage]);

  async function setValueAndPersist(newValue?: T) {
    await sleep(windowStorageSimulatedAccessTimeMs);
    if (newValue !== undefined) {
      const serialized = JSON.stringify(newValue);
      storage.setItem(key, serialized);
      setValue(newValue);
    } else {
      storage.removeItem(key);
      setValue(undefined);
    }
  }

  return [value, setValueAndPersist, isReady];
}

export function useWindowPersistentStorageState<T>(key: string, defaultValue?: T) {
  return useWindowStorageState<T>(window.localStorage, key, defaultValue);
}

export function useWindowSessionStorageState<T>(key: string, defaultValue?: T) {
  return useWindowStorageState<T>(window.sessionStorage, key, defaultValue);
}

// endregion

// region Chrome storage state

// eslint-disable-next-line max-len
export function useBrowserStorageState<T>(
  storage: Storage.StorageArea,
  key: string,
  defaultValue?: T,
): UseStorageStateResult<T> {
  const [value, setValue] = useState<T>();
  const [isReady, setIsReady] = useState<boolean>(false);

  useEffect(() => {
    storage.get(key).then(({ [key]: serialized }) => {
      if (serialized !== undefined) {
        setValue(JSON.parse(serialized));
      } else if (defaultValue !== undefined) {
        setValue(defaultValue);
      }
      setIsReady(true);
    });
  }, [key, defaultValue, setValue, setIsReady, storage]);

  async function setValueAndPersist(newValue?: T) {
    if (newValue !== undefined) {
      const serialized = JSON.stringify(newValue);
      await storage.set({ [key]: serialized });
      setValue(newValue);
    } else {
      await storage.remove(key);
      setValue(undefined);
    }
  }

  return [value, setValueAndPersist, isReady];
}

export function useBrowserPersistentStorageState<T>(key: string, defaultValue?: T) {
  return useBrowserStorageState<T>(chrome.storage.local, key, defaultValue);
}

export function useBrowserSessionStorageState<T>(key: string, defaultValue?: T) {
  const sessionStorage = (chrome.storage as any).session;
  return useBrowserStorageState<T>(sessionStorage, key, defaultValue);
}

// endregion

const hasBrowserStorage = Boolean(chrome.storage);

export const usePersistentStorageState = hasBrowserStorage
  ? useBrowserPersistentStorageState
  : useWindowPersistentStorageState;

export const useSessionStorageState = hasBrowserStorage
  ? useBrowserSessionStorageState
  : useWindowSessionStorageState;
