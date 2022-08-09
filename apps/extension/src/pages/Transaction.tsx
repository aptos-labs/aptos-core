// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import AuthLayout from 'core/layouts/AuthLayout';
import WalletLayout from 'core/layouts/WalletLayout';
import React, { Suspense } from 'react';
import { Routes as PageRoutes } from 'core/routes';
import TransactionBody from 'core/components/TransactionBody';

function Transaction() {
  return (
    <AuthLayout routePath={PageRoutes.transaction.path}>
      <WalletLayout showBackButton>
        <Suspense>
          <TransactionBody />
        </Suspense>
      </WalletLayout>
    </AuthLayout>
  );
}

export default Transaction;
