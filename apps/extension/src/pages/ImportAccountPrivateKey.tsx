// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback } from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import { ImportAccountPrivateKeyLayout, PrivateKeyFormValues } from 'core/layouts/AddAccountLayout';
import ImportAccountPrivateKeyBody from 'core/components/ImportAccountPrivateKeyBody';
import { AptosAccount } from 'aptos';
import { getAccountResources } from 'core/queries/account';
import { useNavigate } from 'react-router-dom';
import { useWalletState } from 'core/hooks/useWalletState';
import { importAccountErrorToast, importAccountNotFoundToast } from 'core/components/Toast';

export default function ImportAccountPrivateKey() {
  const navigate = useNavigate();
  const { addAccount, nodeUrl } = useWalletState();

  const onSubmit = useCallback(async (
    data: PrivateKeyFormValues,
    event?: React.BaseSyntheticEvent,
  ) => {
    const { privateKey } = data;
    event?.preventDefault();
    try {
      const nonHexKey = (privateKey.startsWith('0x')) ? privateKey.substring(2) : privateKey;
      const encodedKey = Uint8Array.from(Buffer.from(nonHexKey, 'hex'));
      const account = new AptosAccount(encodedKey, undefined);
      const response = await getAccountResources({
        address: account.address().hex(),
        nodeUrl,
      });
      if (!response) {
        importAccountNotFoundToast();
        return;
      }
      await addAccount({ account, isImport: true });
      navigate(Routes.wallet.routePath);
    } catch (err) {
      importAccountErrorToast();
    }
  }, [addAccount, navigate, nodeUrl]);

  return (
    <AuthLayout routePath={PageRoutes.importWalletPrivateKey.routePath}>
      <ImportAccountPrivateKeyLayout
        headerValue="Import private key"
        backPage={Routes.addAccount.routePath}
        defaultValues={{
          privateKey: '',
        }}
        onSubmit={onSubmit}
      >
        <ImportAccountPrivateKeyBody />
      </ImportAccountPrivateKeyLayout>
    </AuthLayout>
  );
}
