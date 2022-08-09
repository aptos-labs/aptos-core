// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import constate from 'constate';
import useNetworks from 'core/hooks/useNetworks';
import useAccounts from 'core/hooks/useAccounts';

export function useGlobalState() {
  return {
    ...useAccounts(),
    ...useNetworks(),
  };
}

export * from 'core/hooks/useAccounts';
export * from 'core/hooks/useNetworks';

export const [GlobalStateProvider, useGlobalStateContext] = constate(useGlobalState);
export default useGlobalStateContext;
