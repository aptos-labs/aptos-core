// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Navigate } from 'react-router-dom';
import Account from 'pages/Account';
import Activity from 'pages/Activity';
import CreateWallet from 'pages/CreateWallet';
import SecurityPrivacy from 'pages/SecurityPrivacy';
import Gallery from 'pages/Gallery';
import Help from 'pages/Help';
import Network from 'pages/Network';
import Password from 'pages/Password';
import Settings from 'pages/Settings';
import SwitchAccount from 'pages/SwitchAccount';
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
import Stake from 'pages/Stake';
import ChangePassword from 'pages/ChangePassword';
import AutoLockTimer from 'pages/AutoLockTimer';
import Welcome from 'pages/Welcome';
import Reauthenticate from 'pages/Reauthenticate';
import ExportPublicPrivateKey from 'pages/ExportPublicPrivateKey';
import RemoveAccount from 'pages/RemoveAccount';
import {
  ActiveAccountGuard,
  InitializedAccountsGuard,
  LockedAccountsGuard,
  UnlockedAccountsGuard,
} from './guards';

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
  autolock_timer: {
    element: <AutoLockTimer />,
    path: '/settings/security_privacy/autolock_timer',
  },
  change_password: {
    element: <ChangePassword />,
    path: '/settings/security_privacy/change_password',
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
  export_public_private_key: {
    element:
  <Reauthenticate title="Export Keys">
    <ExportPublicPrivateKey />
  </Reauthenticate>,
    path: '/settings/export_public_private_key',
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
    element: (
      <Reauthenticate title="Show Secret Recovery Phrase">
        <RecoveryPhrase />
      </Reauthenticate>
    ),
    path: '/settings/recovery_phrase',
  },
  remove_account: {
    element:
  <Reauthenticate title="Remove account">
    <RemoveAccount />
  </Reauthenticate>,
    path: '/remove-account',
  },
  rename_account: {
    element: <RenameAccount />,
    path: '/settings/rename_account',
  },
  security_privacy: {
    element: <SecurityPrivacy />,
    path: '/settings/security_privacy',
  },
  settings: {
    element: <Settings />,
    path: '/settings',
  },
  stake: {
    element: <Stake />,
    path: '/stake',
  },
  switchAccount: {
    element: <SwitchAccount />,
    path: '/switch-account',
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
  welcome: {
    element: <Welcome />,
    path: '/welcome',
  },
} as const);

export type RoutePath = typeof Routes[keyof typeof Routes]['path'];
export default Routes;

/**
 * Routes definition for the extension app.
 * At routing time, the router will go through the routes and stop at the first match.
 * Once a match is found, the full tree of components will be rendered.
 *
 * The guard+provider pattern ensures that a specific condition is verified
 * before rendering its children. When the condition is verified,
 * the resolved state is provided to the children which can use it freely without unwrapping,
 * otherwise an appropriate redirect is triggered
 */
export const routes = [
  {
    children: [
      {
        children: [
          {
            children: [
              Routes.autolock_timer,
              Routes.wallet,
              Routes.gallery,
              Routes.token,
              Routes.activity,
              Routes.transaction,
              Routes.remove_account,
              Routes.export_public_private_key,
              Routes.account,
              Routes.settings,
              Routes.switchAccount,
              Routes.change_password,
              Routes.rename_account,
              Routes.network,
              Routes.addNetwork,
              Routes.recovery_phrase,
              Routes.security_privacy,
              Routes.help,
              Routes.stake,
              Routes.welcome,
              { element: <Navigate to={Routes.wallet.path} replace />, path: '/' },
            ],
            element: <ActiveAccountGuard />,
          },
          Routes.addAccount,
          Routes.createAccount,
          Routes.importWalletMnemonic,
          Routes.importWalletPrivateKey,
        ],
        element: <UnlockedAccountsGuard />,
      },
      {
        children: [
          Routes.password,
        ],
        element: <LockedAccountsGuard />,
      },
    ],
    element: <InitializedAccountsGuard />,
  },
  Routes.noWallet,
  Routes.createWallet,
  Routes.createWalletViaImportAccount,
];
