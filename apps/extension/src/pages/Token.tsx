// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import TokenBody from 'core/components/TokenBody';
import WalletLayout from 'core/layouts/WalletLayout';
import React from 'react';

function Token() {
  return (
    <WalletLayout title="Token" showBackButton>
      <TokenBody />
    </WalletLayout>
  );
}

export default Token;
