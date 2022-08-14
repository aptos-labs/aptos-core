// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  ActiveAccountProvider,
  InitializedAccountsProvider,
  UnlockedAccountsProvider,
  useAccounts,
} from 'core/hooks/useAccounts';

import { Navigate, Outlet } from 'react-router-dom';
import { Routes } from 'core/routes';

/**
 * Ensures the accounts state is initialized, or redirect to welcome page.
 * Will provide the initialized accounts for children
 */
export function InitializedAccountsGuard() {
  const { encryptedAccounts, salt } = useAccounts();

  const areAccountsInitialized = encryptedAccounts !== undefined && salt !== undefined;
  return areAccountsInitialized ? (
    <InitializedAccountsProvider encryptedAccounts={encryptedAccounts} salt={salt}>
      <Outlet />
    </InitializedAccountsProvider>
  ) : <Navigate to={Routes.noWallet.path} />;
}

/**
 * Ensures the accounts state is unlocked, or redirect to unlock page.
 * Will provide the unlocked accounts for children
 */
export function UnlockedAccountsGuard() {
  const { accounts, encryptionKey } = useAccounts();
  const isUnlocked = encryptionKey !== undefined && accounts !== undefined;

  return isUnlocked ? (
    <UnlockedAccountsProvider accounts={accounts} encryptionKey={encryptionKey}>
      <Outlet />
    </UnlockedAccountsProvider>
  ) : <Navigate to={Routes.password.path} />;
}

/**
 * Ensures the accounts state is locked, or redirect to home page.
 * This is used for automatic redirection after unlocking
 */
export function LockedAccountsGuard() {
  const { accounts, encryptionKey } = useAccounts();
  const isLocked = encryptionKey === undefined || accounts === undefined;

  return isLocked
    ? <Outlet />
    : <Navigate to={Routes.wallet.path} />;
}

/**
 * Ensures the accounts state contains at least one account,
 * or redirect to account creation page.
 * Will provide the active account for children
 */
export function ActiveAccountGuard() {
  const { activeAccountAddress } = useAccounts();
  const hasActiveAccount = activeAccountAddress !== undefined;

  return hasActiveAccount ? (
    <ActiveAccountProvider activeAccountAddress={activeAccountAddress}>
      <Outlet />
    </ActiveAccountProvider>
  ) : <Navigate to={Routes.addAccount.path} />;
}
