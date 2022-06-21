// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import TokenBody from 'core/components/TokenBody';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import AuthLayout from 'core/layouts/AuthLayout';
import WalletLayout from 'core/layouts/WalletLayout';
import React from 'react';

function Token() {
  return (
    <AuthLayout redirectPath="/">
      <WalletLayout backPage="/gallery">
        <TokenBody />
      </WalletLayout>
    </AuthLayout>

  );
}

export default withSimulatedExtensionContainer({ Component: Token });
