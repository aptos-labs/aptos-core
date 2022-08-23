// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import WalletLayout from 'core/layouts/WalletLayout';
import React from 'react';
import TransactionBody from 'core/components/TransactionBody';

function Transaction() {
  return (
    <WalletLayout title="Transaction" showBackButton>
      <TransactionBody />
    </WalletLayout>
  );
}

export default Transaction;
