// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import ImportWalletLayout from 'core/layouts/ImportWalletLayout';
import ImportAccountPrivateKeyBody from 'core/components/ImportAccountPrivateKeyBody';

export default function ImportAccountPrivateKey() {
  return (
    <AuthLayout routePath={PageRoutes.createWallet.routePath}>
      <ImportWalletLayout headerValue="Import private key" backPage="/import">
        <ImportAccountPrivateKeyBody />
      </ImportWalletLayout>
    </AuthLayout>
  );
}
