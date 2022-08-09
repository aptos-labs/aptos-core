// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback } from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import ImportAccountMnemonicBody from 'core/components/ImportAccountMnemonicBody';
import { ImportAccountMnemonicLayout, MnemonicFormValues } from 'core/layouts/AddAccountLayout';
import useWalletState from 'core/hooks/useWalletState';
import { useNavigate } from 'react-router-dom';
import { generateMnemonicObject } from 'core/utils/account';
import { AptosAccount } from 'aptos';
import { getAccountResources } from 'core/queries/account';
import { importAccountErrorToast, importAccountNotFoundToast } from 'core/components/Toast';

export default function ImportWalletMnemonic() {
  const navigate = useNavigate();
  const { addAccount, nodeUrl } = useWalletState();

  const onSubmit = useCallback(async (
    mnemonicAll: MnemonicFormValues,
    event?: React.BaseSyntheticEvent,
  ) => {
    event?.preventDefault();
    let mnemonicString = '';
    Object.values(mnemonicAll).forEach((value) => {
      mnemonicString = `${mnemonicString + value} `;
    });
    mnemonicString = mnemonicString.trim();
    try {
      const mnemonicObject = await generateMnemonicObject(mnemonicString);
      const aptosAccount = new AptosAccount(mnemonicObject.seed);
      const response = await getAccountResources({
        address: aptosAccount.address().hex(),
        nodeUrl,
      });
      if (!response) {
        // invalid mneomic, not found
        importAccountNotFoundToast();
        return;
      }
      await addAccount({ account: aptosAccount, isImport: true, mnemonic: mnemonicObject });
      navigate(Routes.wallet.routePath);
    } catch (err) {
      importAccountErrorToast();
      // eslint-disable-next-line no-console
      console.error('Invalid mnemonic, account not found');
    }
  }, [addAccount, navigate, nodeUrl]);

  return (
    <AuthLayout routePath={PageRoutes.importWalletMnemonic.routePath}>
      <ImportAccountMnemonicLayout
        headerValue="Import mnemonic"
        backPage={Routes.addAccount.routePath}
        defaultValues={{}}
        onSubmit={onSubmit}
      >
        <ImportAccountMnemonicBody />
      </ImportAccountMnemonicLayout>
    </AuthLayout>
  );
}
