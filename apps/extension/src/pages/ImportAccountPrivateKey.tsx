// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback } from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import { ImportAccountPrivateKeyLayout, PrivateKeyFormValues } from 'core/layouts/AddAccountLayout';
import ImportAccountPrivateKeyBody from 'core/components/ImportAccountPrivateKeyBody';
import { AptosAccount } from 'aptos';
import { useNavigate } from 'react-router-dom';
import { importAccountErrorToast, importAccountToast } from 'core/components/Toast';
import useGlobalStateContext from 'core/hooks/useGlobalState';

export default function ImportAccountPrivateKey() {
  const navigate = useNavigate();
  const { addAccount, newAccountStyleIndex } = useGlobalStateContext();

  const onSubmit = useCallback(async (
    data: PrivateKeyFormValues,
    event?: React.BaseSyntheticEvent,
  ) => {
    const { privateKey } = data;
    event?.preventDefault();
    try {
      const nonHexKey = (privateKey.startsWith('0x')) ? privateKey.substring(2) : privateKey;
      const encodedKey = Uint8Array.from(Buffer.from(nonHexKey, 'hex'));
      const aptosAccount = new AptosAccount(encodedKey);
      // TODO: prompt user for confirmation if account is not on chain

      const {
        address,
        privateKeyHex,
        publicKeyHex,
      } = aptosAccount.toPrivateKeyObject();

      await addAccount({
        address: address!,
        name: 'Wallet',
        privateKey: privateKeyHex,
        publicKey: publicKeyHex!,
        styleIndex: newAccountStyleIndex ?? 0,
      });

      importAccountToast();
      navigate(Routes.wallet.path);
    } catch (err) {
      importAccountErrorToast();
    }
  }, [addAccount, navigate, newAccountStyleIndex]);

  return (
    <AuthLayout routePath={PageRoutes.importWalletPrivateKey.path}>
      <ImportAccountPrivateKeyLayout
        headerValue="Import private key"
        backPage={Routes.addAccount.path}
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
