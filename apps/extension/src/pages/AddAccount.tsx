// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import AddAccountBody from 'core/components/AddAccountBody';
import ImportAccountLayout from 'core/layouts/ImportAccountLayout';

export default function AddAccount() {
  return (
    <AuthLayout routePath={PageRoutes.addAccount.path}>
      <ImportAccountLayout backPage={Routes.wallet.path}>
        <AddAccountBody />
      </ImportAccountLayout>
    </AuthLayout>
  );
}
