// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import AuthLayout from 'core/layouts/AuthLayout';
import WalletLayout from 'core/layouts/WalletLayout';
import React, { Suspense } from 'react';
import { Routes as PageRoutes } from 'core/routes';
import TransactionBody from 'core/components/TransactionBody';

function Transaction() {
  return (
    <AuthLayout routePath={PageRoutes.transaction.routePath}>
      <WalletLayout backPage="/activity">
        <Suspense>
          <TransactionBody />
        </Suspense>
      </WalletLayout>
    </AuthLayout>
  );
}

export default withSimulatedExtensionContainer({ Component: Transaction });
