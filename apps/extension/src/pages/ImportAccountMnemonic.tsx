// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import ImportAccountMnemonicBody from 'core/components/ImportAccountMnemonicBody';
import ImportAccountLayout from 'core/layouts/ImportAccountLayout';

export default function ImportWalletMnemonic() {
  return (
    <AuthLayout routePath={PageRoutes.createWallet.routePath}>
      <ImportAccountLayout headerValue="Import mnemonic" backPage={Routes.addAccount.routePath}>
        <ImportAccountMnemonicBody />
      </ImportAccountLayout>
    </AuthLayout>
  );
}
