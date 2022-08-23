// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo, useState } from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import CreateAccountBody from 'core/components/CreateAccountBody';
import { CreateAccountFormValues, CreateAccountLayout } from 'core/layouts/AddAccountLayout';
import { useNavigate } from 'react-router-dom';
import { AptosAccount } from 'aptos';
import { generateMnemonic, generateMnemonicObject } from 'core/utils/account';
import useGlobalStateContext from 'core/hooks/useGlobalState';
import useFundAccount from 'core/mutations/faucet';
import { createAccountErrorToast, createAccountToast } from 'core/components/Toast';

function CreateAccount() {
  const navigate = useNavigate();
  const {
    addAccount,
    faucetClient,
    newAccountStyleIndex,
  } = useGlobalStateContext();
  const { fundAccount } = useFundAccount();
  const newMnemonic = useMemo(() => generateMnemonic(), []);
  const [isLoading, setIsLoading] = useState<boolean>(false);

  const onSubmit = async (data: CreateAccountFormValues, event?: React.BaseSyntheticEvent) => {
    const { mnemonicString, secretRecoveryPhrase } = data;
    event?.preventDefault();
    setIsLoading(true);

    if (secretRecoveryPhrase) {
      try {
        const { mnemonic, seed } = await generateMnemonicObject(mnemonicString);
        const aptosAccount = new AptosAccount(seed);
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

        if (faucetClient) {
          await fundAccount({ address: address!, amount: 0 });
        }

        createAccountToast();
        navigate(Routes.wallet.path);
      } catch (err) {
        createAccountErrorToast();
        // eslint-disable-next-line no-console
        console.error(err);
      }
    }
    setIsLoading(false);
  };

  return (
    <AuthLayout routePath={PageRoutes.createAccount.path}>
      <CreateAccountLayout
        headerValue="Create account"
        backPage={Routes.addAccount.path}
        defaultValues={{
          mnemonic: newMnemonic.split(' '),
          mnemonicString: newMnemonic,
          secretRecoveryPhrase: false,
        }}
        onSubmit={onSubmit}
      >
        <CreateAccountBody isLoading={isLoading} />
      </CreateAccountLayout>
    </AuthLayout>
  );
}

export default CreateAccount;
