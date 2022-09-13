// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  ActiveAccountProvider,
  InitializedAccountsProvider,
  UnlockedAccountsProvider,
  useAccounts,
  useUnlockedAccounts,
} from 'core/hooks/useAccounts';

import { Navigate, Outlet } from 'react-router-dom';
import { Routes } from 'core/routes';

export interface RouteGuardProps {
  redirectTo?: string,
}

/**
 * Ensures the accounts state is initialized, or redirect to welcome page.
 * Will provide the initialized accounts for children
 */
export function InitializedAccountsGuard({ redirectTo }: RouteGuardProps) {
  const { encryptedAccounts, encryptedStateVersion, salt } = useAccounts();

  const areAccountsInitialized = encryptedAccounts !== undefined && salt !== undefined;
  return areAccountsInitialized ? (
    <InitializedAccountsProvider
      encryptedAccounts={encryptedAccounts}
      salt={salt}
      encryptedStateVersion={encryptedStateVersion ?? 0}
    >
      <Outlet />
    </InitializedAccountsProvider>
  ) : <Navigate to={redirectTo ?? Routes.noWallet.path} />;
}

/**
 * Ensures the accounts state is unlocked, or redirect to unlock page.
 * Will provide the unlocked accounts for children
 */
export function UnlockedAccountsGuard({ redirectTo }: RouteGuardProps) {
  const { accounts, encryptionKey } = useAccounts();
  const isUnlocked = encryptionKey !== undefined && accounts !== undefined;

  return isUnlocked ? (
    <UnlockedAccountsProvider accounts={accounts} encryptionKey={encryptionKey}>
      <Outlet />
    </UnlockedAccountsProvider>
  ) : <Navigate to={redirectTo ?? Routes.password.path} />;
}

/**
 * Ensures the accounts state is locked, or redirect to home page.
 * This is used for automatic redirection after unlocking
 */
export function LockedAccountsGuard({ redirectTo }: RouteGuardProps) {
  const { accounts, encryptionKey } = useAccounts();
  const isLocked = encryptionKey === undefined || accounts === undefined;

  return isLocked
    ? <Outlet />
    : <Navigate to={redirectTo ?? Routes.wallet.path} />;
}

/**
 * Ensures the accounts state contains at least one account,
 * or redirect to account creation page.
 * Will provide the active account for children
 */
export function ActiveAccountGuard({ redirectTo }: RouteGuardProps) {
  const { activeAccountAddress } = useAccounts();
  const { accounts } = useUnlockedAccounts();

  // Fall back to first available account if activeAccount was just removed
  const isActiveAccountAvailable = activeAccountAddress !== undefined
    && activeAccountAddress in accounts;
  const activeOrFirstAccountAddress = isActiveAccountAvailable
    ? activeAccountAddress
    : Object.keys(accounts)[0];
  const hasActiveAccount = activeOrFirstAccountAddress !== undefined;

  return hasActiveAccount ? (
    <ActiveAccountProvider activeAccountAddress={activeOrFirstAccountAddress}>
      <Outlet />
    </ActiveAccountProvider>
  ) : <Navigate to={redirectTo ?? Routes.addAccount.path} />;
}
