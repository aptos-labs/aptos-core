// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import ImportAccountLayout from 'core/layouts/ImportAccountLayout';
import ImportAccountPrivateKeyBody from 'core/components/ImportAccountPrivateKeyBody';

export default function ImportAccountPrivateKey() {
  return (
    <AuthLayout routePath={PageRoutes.createWallet.routePath}>
      <ImportAccountLayout headerValue="Import private key" backPage={Routes.addAccount.routePath}>
        <ImportAccountPrivateKeyBody />
      </ImportAccountLayout>
    </AuthLayout>
  );
}
