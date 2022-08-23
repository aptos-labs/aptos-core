// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import Routes from 'core/routes';
import AddAccountBody from 'core/components/AddAccountBody';
import ImportAccountLayout from 'core/layouts/ImportAccountLayout';
import { useAccounts } from 'core/hooks/useAccounts';

export default function AddAccount() {
  const { activeAccountAddress } = useAccounts();
  const backPage = activeAccountAddress ? Routes.wallet.path : undefined;

  return (
    <ImportAccountLayout backPage={backPage}>
      <AddAccountBody />
    </ImportAccountLayout>
  );
}
