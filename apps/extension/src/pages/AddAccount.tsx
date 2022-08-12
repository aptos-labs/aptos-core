// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import AddAccountBody from 'core/components/AddAccountBody';
import ImportAccountLayout from 'core/layouts/ImportAccountLayout';
import useGlobalStateContext from 'core/hooks/useGlobalState';

export default function AddAccount() {
  const { activeAccountAddress } = useGlobalStateContext();
  const backPage = activeAccountAddress ? Routes.wallet.path : undefined;

  return (
    <AuthLayout routePath={PageRoutes.addAccount.path}>
      <ImportAccountLayout backPage={backPage}>
        <AddAccountBody />
      </ImportAccountLayout>
    </AuthLayout>
  );
}
