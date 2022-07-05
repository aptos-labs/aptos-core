// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import Activity from 'pages/Activity';
import CreateWallet from 'pages/CreateWallet';
import Credentials from 'pages/Credentials';
import Gallery from 'pages/Gallery';
import Help from 'pages/Help';
import Login from 'pages/Login';
import Network from 'pages/Network';
import Password from 'pages/Password';
import Settings from 'pages/Settings';
import Token from 'pages/Token';
import Wallet from 'pages/Wallet';
import React from 'react';

export const Routes = Object.freeze({
  activity: {
    element: <Activity />,
    routePath: '/activity',
  },
  createWallet: {
    element: <CreateWallet />,
    routePath: '/create-wallet',
  },
  credentials: {
    element: <Credentials />,
    routePath: '/settings/credentials',
  },
  gallery: {
    element: <Gallery />,
    routePath: '/gallery',
  },
  help: {
    element: <Help />,
    routePath: '/help',
  },
  login: {
    element: <Login />,
    routePath: '/',
  },
  network: {
    element: <Network />,
    routePath: '/settings/network',
  },
  password: {
    element: <Password />,
    routePath: '/password',
  },
  settings: {
    element: <Settings />,
    routePath: '/settings',
  },
  token: {
    element: <Token />,
    routePath: '/tokens/:id',
  },
  wallet: {
    element: <Wallet />,
    routePath: '/wallet',
  },
} as const);

export type RoutePaths = typeof Routes[keyof typeof Routes]['routePath'];

export default Routes;
