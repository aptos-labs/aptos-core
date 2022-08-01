// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import ImportAccountMnemonicBody from 'core/components/ImportAccountMnemonicBody';
import ImportWalletLayout from 'core/layouts/ImportWalletLayout';

export default function ImportWalletMnemonic() {
  return (
    <AuthLayout routePath={PageRoutes.createWallet.routePath}>
      <ImportWalletLayout headerValue="Import mnemonic" backPage="/import">
        <ImportAccountMnemonicBody />
      </ImportWalletLayout>
    </AuthLayout>
  );
}
