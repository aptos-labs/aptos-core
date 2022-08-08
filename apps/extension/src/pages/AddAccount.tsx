// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import AddAccountBody from 'core/components/AddAccountBody';
import ImportAccountLayout from 'core/layouts/ImportAccountLayout';

export default function AddAccount() {
  return (
    <AuthLayout routePath={PageRoutes.createWallet.routePath}>
      <ImportAccountLayout backPage="/">
        <AddAccountBody />
      </ImportAccountLayout>
    </AuthLayout>
  );
}
