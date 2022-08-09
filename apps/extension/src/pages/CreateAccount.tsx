// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import CreateAccountBody from 'core/components/CreateAccountBody';
import { CreateAccountFormValues, CreateAccountLayout } from 'core/layouts/AddAccountLayout';
import { useNavigate } from 'react-router-dom';
import { AptosAccount } from 'aptos';
import { generateMnemonic, generateMnemonicObject } from 'core/utils/account';
import { useWalletState } from 'core/hooks/useWalletState';

function CreateAccount() {
  const navigate = useNavigate();
  const { addAccount } = useWalletState();
  const mnemonic = useMemo(() => generateMnemonic(), []);

  const onSubmit = async (data: CreateAccountFormValues, event?: React.BaseSyntheticEvent) => {
    const { mnemonicString, secretRecoveryPhrase } = data;
    event?.preventDefault();

    if (secretRecoveryPhrase) {
      try {
        const mnemonicObject = await generateMnemonicObject(mnemonicString);
        const aptosAccount = new AptosAccount(mnemonicObject.seed);
        await addAccount({ account: aptosAccount, isImport: false, mnemonic: mnemonicObject });
        navigate(Routes.wallet.routePath);
      } catch (err) {
        // eslint-disable-next-line no-console
        console.error(err);
      }
    }
  };

  return (
    <AuthLayout routePath={PageRoutes.createAccount.routePath}>
      <CreateAccountLayout
        headerValue="Create account"
        backPage={Routes.addAccount.routePath}
        defaultValues={{
          mnemonic: mnemonic.split(' '),
          mnemonicString: mnemonic,
          secretRecoveryPhrase: false,
        }}
        onSubmit={onSubmit}
      >
        <CreateAccountBody />
      </CreateAccountLayout>
    </AuthLayout>
  );
}

export default CreateAccount;
