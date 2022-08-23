// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import TokenBody from 'core/components/TokenBody';
import AuthLayout from 'core/layouts/AuthLayout';
import WalletLayout from 'core/layouts/WalletLayout';
import React from 'react';
import { Routes as PageRoutes } from 'core/routes';

function Token() {
  return (
    <AuthLayout routePath={PageRoutes.token.path}>
      <WalletLayout title="Token" showBackButton>
        <TokenBody />
      </WalletLayout>
    </AuthLayout>
  );
}

export default Token;
