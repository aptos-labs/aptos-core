/* eslint-disable sort-keys-fix/sort-keys-fix,sort-keys */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Navigate } from 'react-router-dom';
import React from 'react';

import {
  ActiveAccountGuard,
  InitializedAccountsGuard,
  LockedAccountsGuard,
  UnlockedAccountsGuard,
} from 'core/guards';

import Password from 'pages/Password';
import {
  PermissionsPrompt,
  NoAccounts,
} from './pages';

export const Routes = {
  noAccounts: { path: 'no-accounts', element: <NoAccounts /> },
  unlock: { path: 'unlock', element: <Password /> },
  request: { path: 'request', element: <PermissionsPrompt /> },
};

/**
 * Routes definition for the extension prompt.
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
    element: <InitializedAccountsGuard redirectTo={Routes.noAccounts.path} />,
    children: [
      {
        element: <UnlockedAccountsGuard redirectTo={Routes.unlock.path} />,
        children: [
          {
            element: <ActiveAccountGuard redirectTo={Routes.noAccounts.path} />,
            children: [
              Routes.request,
              { element: <Navigate to={Routes.request.path} replace />, path: '/' },
            ],
          },
        ],
      },
      {
        element: <LockedAccountsGuard redirectTo={Routes.request.path} />,
        children: [
          { path: 'unlock', element: <Password /> },
        ],
      },
    ],
  },
  Routes.noAccounts,
];
