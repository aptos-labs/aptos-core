// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useAppState } from 'core/hooks/useAppState';

export default function useAutoLock() {
  const {
    updatePersistentState,
  } = useAppState();

  const updateAutoLock = async (timerInMinutes: number = 15) => {
    // update the persistent state with new timer
    await updatePersistentState({
      autolockTimer: timerInMinutes,
    });
  };

  return {
    updateAutoLock,
  };
}
