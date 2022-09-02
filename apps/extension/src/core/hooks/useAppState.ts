// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import constate from 'constate';
import { useEffect, useState } from 'react';
import { PersistentStorage, SessionStorage } from 'shared/storage';
import { PersistentState, SessionState } from 'shared/types';

/**
 * Hook/provider for the app global state.
 * The state is split into persistent and session state, which are mapped respectively to
 * PersistentStorage and SessionStorage (cleared when the browser session ends).
 * The underlying storage is async in nature, thus the consumer needs to wait for
 * the `isAppStateReady` flag to be set before using the state.
 */
export const [AppStateProvider, useAppState] = constate(() => {
  const [persistentState, setPersistentState] = useState<PersistentState>();
  const [sessionState, setSessionState] = useState<SessionState>();
  const [isAppStateReady, setIsAppStateReady] = useState<boolean>(false);

  useEffect(() => {
    Promise.all([
      PersistentStorage.get([
        'activeAccountAddress',
        'activeAccountPublicKey',
        'activeNetworkName',
        'customNetworks',
        'encryptedAccounts',
        'salt',
        'encryptedStateVersion',
      ]),
      SessionStorage.get([
        'accounts',
        'encryptionKey',
      ]),
    ]).then(([initialPersistentState, initialSessionState]) => {
      setPersistentState(initialPersistentState);
      setSessionState(initialSessionState);
      setIsAppStateReady(true);
    });
  }, []);

  const updatePersistentState = async (newValues: Partial<PersistentState>) => {
    await PersistentStorage.set(newValues);
    const newPersistentState = { ...persistentState, ...newValues } as PersistentState;
    setPersistentState(newPersistentState);
  };

  const updateSessionState = async (newValues: Partial<SessionState>) => {
    await SessionStorage.set(newValues);
    const newSessionState = { ...sessionState, ...newValues } as SessionState;
    setSessionState(newSessionState);
  };

  return {
    ...persistentState,
    ...sessionState,
    isAppStateReady,
    updatePersistentState,
    updateSessionState,
  };
});
