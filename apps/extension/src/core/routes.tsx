// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Navigate } from 'react-router-dom';
import Account from 'pages/Account';
import Activity from 'pages/Activity';
import CreateWallet from 'pages/CreateWallet';
import Credentials from 'pages/Credentials';
import Gallery from 'pages/Gallery';
import Help from 'pages/Help';
import Network from 'pages/Network';
import Password from 'pages/Password';
import Settings from 'pages/Settings';
import Token from 'pages/Token';
import Wallet from 'pages/Wallet';
import React from 'react';
import RecoveryPhrase from 'pages/RecoveryPhrase';
import Transaction from 'pages/Transaction';
import NoWallet from 'pages/NoWallet';
import AddAccount from 'pages/AddAccount';
import ImportAccountMnemonic from 'pages/ImportAccountMnemonic';
import ImportAccountPrivateKey from 'pages/ImportAccountPrivateKey';
import CreateAccount from 'pages/CreateAccount';
import AddNetwork from 'pages/AddNetwork';
import RenameAccount from 'pages/RenameAccount';
import CreateWalletViaImportAccount from 'pages/CreateWalletViaImportAccount';

// TODO: have a single representation for routes

export const Routes = Object.freeze({
  account: {
    element: <Account />,
    path: '/accounts/:address',
  },
  activity: {
    element: <Activity />,
    path: '/activity',
  },
  addAccount: {
    element: <AddAccount />,
    path: '/add-account',
  },
  addNetwork: {
    element: <AddNetwork />,
    path: '/settings/add-network',
  },
  createAccount: {
    element: <CreateAccount />,
    path: '/create-account',
  },
  createWallet: {
    element: <CreateWallet />,
    path: '/create-wallet',
  },
  createWalletViaImportAccount: {
    element: <CreateWalletViaImportAccount />,
    path: '/create-wallet/import-account',
  },
  credentials: {
    element: <Credentials />,
    path: '/settings/credentials',
  },
  gallery: {
    element: <Gallery />,
    path: '/gallery',
  },
  help: {
    element: <Help />,
    path: '/help',
  },
  importWalletMnemonic: {
    element: <ImportAccountMnemonic />,
    path: '/import/mnemonic',
  },
  importWalletPrivateKey: {
    element: <ImportAccountPrivateKey />,
    path: '/import/private-key',
  },
  login: {
    element: <NoWallet />,
    path: '/',
  },
  network: {
    element: <Network />,
    path: '/settings/network',
  },
  noWallet: {
    element: <NoWallet />,
    path: '/no-wallet',
  },
  password: {
    element: <Password />,
    path: '/password',
  },
  recovery_phrase: {
    element: <RecoveryPhrase />,
    path: '/settings/recovery_phrase',
  },
  rename_account: {
    element: <RenameAccount />,
    path: '/settings/rename_account',
  },
  settings: {
    element: <Settings />,
    path: '/settings',
  },
  token: {
    element: <Token />,
    path: '/tokens/:id',
  },
  transaction: {
    element: <Transaction />,
    path: '/transactions/:version',
  },
  wallet: {
    element: <Wallet />,
    path: '/wallet',
  },
} as const);

export type RoutePaths = typeof Routes[keyof typeof Routes]['path'];

export default Routes;

export const mainRoutes = Object.freeze([
  Routes.wallet,
  Routes.gallery,
  Routes.token,
  Routes.activity,
  Routes.transaction,
  Routes.account,
  Routes.settings,
  Routes.rename_account,
  Routes.credentials,
  Routes.network,
  Routes.recovery_phrase,
  Routes.createAccount,
  Routes.addAccount,
  Routes.addNetwork,
  Routes.importWalletMnemonic,
  Routes.importWalletPrivateKey,
  Routes.help,
  // this needs to be here to prevent force redirect on last screen of wallet creation
  Routes.createWallet,
  { element: <Navigate to={Routes.wallet.path} replace />, path: '*' },
]);

export const noAccountsRoutes = Object.freeze([
  Routes.addAccount,
  Routes.createAccount,
  {
    element: <Navigate to={Routes.addAccount.path} replace />,
    path: '*',
  },
]);

export const lockedWalletRoutes = Object.freeze([
  Routes.password,
  {
    element: <Navigate to={Routes.password.path} replace />,
    path: '*',
  },
]);

export const uninitializedRoutes = Object.freeze([
  Routes.noWallet,
  Routes.createWalletViaImportAccount,
  Routes.createWallet,
  {
    element: <Navigate to={Routes.noWallet.path} replace />,
    path: '*',
  },
]);
