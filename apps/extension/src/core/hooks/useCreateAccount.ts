// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, HexString, AptosClient } from 'aptos';
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

  const lookupOriginalAddress = async (
    aptosClient: AptosClient,
    aptosAccount: AptosAccount,
    mnemonic?: string,
  ) => {
    // Attempt to look up original address to see
    // if account key has been rotated previously
    const originalAddress: HexString = await aptosClient.lookupOriginalAddress(
      aptosAccount.address(),
    );

    // if account is looked up successfully, it means account key has been rotated
    // therefore update the account derived from private key
    // with the original address
    const newAptosAccount = AptosAccount.fromAptosAccountObject({
      ...aptosAccount.toPrivateKeyObject(),
      address: HexString.ensure(originalAddress).toString(),
    });

    // pass in mnemonic phrase if account is imported via secret recovery phrase
    const newAccount = mnemonic ? {
      mnemonic,
      ...keysFromAptosAccount(newAptosAccount),
    } : keysFromAptosAccount(newAptosAccount);

    return newAccount;
  };

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

  return { createAccount, lookupOriginalAddress };
};

export default useCreateAccount;
