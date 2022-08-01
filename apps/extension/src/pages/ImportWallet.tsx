// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import ImportWalletBody from 'core/components/ImportWalletBody';
import ImportWalletLayout from 'core/layouts/ImportWalletLayout';

export default function ImportWallet() {
  return (
    <AuthLayout routePath={PageRoutes.createWallet.routePath}>
      <ImportWalletLayout backPage="/">
        <ImportWalletBody />
      </ImportWalletLayout>
    </AuthLayout>
  );
}
