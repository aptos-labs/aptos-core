// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export type StorageChange<T> = {
  newValue?: T,
  oldValue?: T,
};

export type StorageChanges<TState> = {
  [key in keyof TState]?: StorageChange<TState[key]>;
};
