// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo, useState } from 'react';
import Routes from 'core/routes';
import CreateAccountBody from 'core/components/CreateAccountBody';
import { CreateAccountFormValues, CreateAccountLayout } from 'core/layouts/AddAccountLayout';
import { useNavigate } from 'react-router-dom';
import { AptosAccount } from 'aptos';
import { generateMnemonic, generateMnemonicObject, keysFromAptosAccount } from 'core/utils/account';
import { useUnlockedAccounts } from 'core/hooks/useAccounts';
import useFundAccount from 'core/mutations/faucet';
import { createAccountErrorToast, createAccountToast } from 'core/components/Toast';

function CreateAccount() {
  const navigate = useNavigate();
  const { addAccount } = useUnlockedAccounts();
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

        const newAccount = {
          mnemonic,
          ...keysFromAptosAccount(aptosAccount),
        };
        await addAccount(newAccount);

        if (fundAccount) {
          await fundAccount({ address: newAccount.address, amount: 0 });
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
  );
}

export default CreateAccount;
