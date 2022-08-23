// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback } from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import ImportAccountMnemonicBody from 'core/components/ImportAccountMnemonicBody';
import { ImportAccountMnemonicLayout, MnemonicFormValues } from 'core/layouts/AddAccountLayout';
import { useNavigate } from 'react-router-dom';
import { generateMnemonicObject } from 'core/utils/account';
import { AptosAccount } from 'aptos';
import { importAccountErrorToast, importAccountToast } from 'core/components/Toast';
import useGlobalStateContext from 'core/hooks/useGlobalState';

export default function ImportWalletMnemonic() {
  const navigate = useNavigate();
  const { addAccount, newAccountStyleIndex } = useGlobalStateContext();

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
      const { mnemonic, seed } = await generateMnemonicObject(mnemonicString);
      const aptosAccount = new AptosAccount(seed);
      // TODO: prompt user for confirmation if account is not on chain

      const {
        address,
        privateKeyHex,
        publicKeyHex,
      } = aptosAccount.toPrivateKeyObject();

      await addAccount({
        address: address!,
        mnemonic,
        name: 'Wallet',
        privateKey: privateKeyHex,
        publicKey: publicKeyHex!,
        styleIndex: newAccountStyleIndex ?? 0,
      });

      importAccountToast();
      navigate(Routes.wallet.path);
    } catch (err) {
      importAccountErrorToast();
      // eslint-disable-next-line no-console
      console.error('Invalid mnemonic, account not found');
    }
  }, [addAccount, navigate]);

  return (
    <AuthLayout routePath={PageRoutes.importWalletMnemonic.path}>
      <ImportAccountMnemonicLayout
        headerValue="Import mnemonic"
        backPage={Routes.addAccount.path}
        defaultValues={{}}
        onSubmit={onSubmit}
      >
        <ImportAccountMnemonicBody />
      </ImportAccountMnemonicLayout>
    </AuthLayout>
  );
}
