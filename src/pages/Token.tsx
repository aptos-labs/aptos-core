// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import TokenBody from 'core/components/TokenBody';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import WalletLayout from 'core/layouts/WalletLayout';
import React from 'react';

function Token() {
  return (
    <WalletLayout backPage="/gallery">
      <TokenBody />
    </WalletLayout>
  );
}

export default withSimulatedExtensionContainer(Token);
