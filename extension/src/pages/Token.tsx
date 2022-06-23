// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import TokenBody from 'core/components/TokenBody';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import AuthLayout from 'core/layouts/AuthLayout';
import WalletLayout from 'core/layouts/WalletLayout';
import React from 'react';
import { Routes as PageRoutes } from 'core/routes';

function Token() {
  return (
    <AuthLayout routePath={PageRoutes.token.routePath}>
      <WalletLayout backPage="/gallery">
        <TokenBody />
      </WalletLayout>
    </AuthLayout>

  );
}

export default withSimulatedExtensionContainer({ Component: Token });
