// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount } from 'aptos';
import {
  Account,
} from 'shared/types';
import { generateMnemonic, generateMnemonicObject, keysFromAptosAccount } from 'core/utils/account';
import { useUnlockedAccounts } from 'core/hooks/useAccounts';
import useFundAccount from 'core/mutations/faucet';
import { createAccountErrorToast, createAccountToast } from 'core/components/Toast';
import { useAnalytics } from 'core/hooks/useAnalytics';
import { accountEvents } from 'core/utils/analytics/events';

interface UseCreateAccountProps {
  shouldAddAccount?: boolean;
  shouldFundAccount?: boolean;
  shouldShowToast?: boolean;
}
const useCreateAccount = ({
  shouldAddAccount = true,
  shouldFundAccount = true,
  shouldShowToast = false,
}: UseCreateAccountProps) => {
  const { fundAccount } = useFundAccount();
  const { addAccount } = useUnlockedAccounts();
  const { trackEvent } = useAnalytics();

  const createAccount = async (): Promise<Account | undefined> => {
    let newAccount;
    try {
      const newMnemonic = generateMnemonic();
      const { mnemonic, seed } = await generateMnemonicObject(newMnemonic);
      const aptosAccount = new AptosAccount(seed);

      newAccount = {
        mnemonic,
        ...keysFromAptosAccount(aptosAccount),
      };

      if (shouldAddAccount) {
        await addAccount(newAccount);
      }

      if (shouldFundAccount && fundAccount) {
        await fundAccount({ address: newAccount.address, amount: 0 });
      }

      if (shouldShowToast) {
        createAccountToast();
      }

      trackEvent({
        eventType: accountEvents.CREATE_ACCOUNT,
      });
    } catch (err) {
      if (shouldShowToast) {
        createAccountErrorToast();
      }

      // eslint-disable-next-line no-console
      console.error(err);
    }

    return newAccount;
  };

  return { createAccount };
};

export default useCreateAccount;
