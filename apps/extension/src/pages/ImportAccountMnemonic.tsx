// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback } from 'react';
import Routes from 'core/routes';
import ImportAccountMnemonicBody from 'core/components/ImportAccountMnemonicBody';
import { ImportAccountMnemonicLayout, MnemonicFormValues } from 'core/layouts/AddAccountLayout';
import { useNavigate } from 'react-router-dom';
import { generateMnemonicObject, keysFromAptosAccount } from 'core/utils/account';
import { AptosAccount } from 'aptos';
import { importAccountErrorToast, importAccountToast } from 'core/components/Toast';
import { useUnlockedAccounts } from 'core/hooks/useAccounts';

export default function ImportWalletMnemonic() {
  const navigate = useNavigate();
  const { addAccount } = useUnlockedAccounts();

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

      await addAccount({
        mnemonic,
        ...keysFromAptosAccount(aptosAccount),
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
    <ImportAccountMnemonicLayout
      headerValue="Import mnemonic"
      backPage={Routes.addAccount.path}
      defaultValues={{}}
      onSubmit={onSubmit}
    >
      <ImportAccountMnemonicBody />
    </ImportAccountMnemonicLayout>
  );
}
