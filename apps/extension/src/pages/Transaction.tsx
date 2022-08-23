// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import AuthLayout from 'core/layouts/AuthLayout';
import WalletLayout from 'core/layouts/WalletLayout';
import React from 'react';
import { Routes as PageRoutes } from 'core/routes';
import TransactionBody from 'core/components/TransactionBody';

function Transaction() {
  return (
    <AuthLayout routePath={PageRoutes.transaction.path}>
      <WalletLayout title="Transaction" showBackButton>
        <TransactionBody />
      </WalletLayout>
    </AuthLayout>
  );
}

export default Transaction;
