// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosAccount, AptosClient, HexString, ApiError,
} from 'aptos';
import {
  Account,
  Accounts,
} from 'shared/types';
import { keysFromAptosAccount } from 'core/utils/account';

export const lookupOriginalAddress = async (
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

interface LookUpAndInitAccountsProps {
  aptosAccount: AptosAccount,
  aptosClient: AptosClient,
  confirmPassword: string,
  initAccounts: (password: string, initialAccounts: Accounts) => Promise<void>;
  mnemonic?: string,
}

export async function lookUpAndInitAccounts({
  aptosAccount,
  aptosClient,
  confirmPassword,
  initAccounts,
  mnemonic,
}: LookUpAndInitAccountsProps) {
  try {
    const newAccount = await lookupOriginalAddress(aptosClient, aptosAccount, mnemonic);

    await initAccounts(confirmPassword, {
      [newAccount.address]: newAccount,
    });
  } catch (err) {
    // if account failed to be looked up then account key has not been rotated
    // therefore add the account derived from private key or mnemonic string
    // errorCode 'table_item_not_found' means address cannot be found in the table
    if (err instanceof ApiError && err.errorCode === 'table_item_not_found') {
      // eslint-disable-next-line no-console
      console.error('failed to fetch rotated key for address ', aptosAccount.address());

      const newAccount = mnemonic ? {
        mnemonic,
        ...keysFromAptosAccount(aptosAccount),
      } : keysFromAptosAccount(aptosAccount);

      await initAccounts(confirmPassword, {
        [newAccount.address]: newAccount,
      });
    } else {
      // throw err here so we can catch it later in Import Account flow
      // and raise error toast/trigger analytic event
      throw err;
    }
  }
}

interface LookUpAndAddAccountProps {
  addAccount: (account: Account) => Promise<void>;
  aptosAccount: AptosAccount;
  aptosClient: AptosClient;
  mnemonic?: string;
}

// look up account on chain to determine if account has rotated private key
// 1. if account has private key rotated,
// update the derived account with the original account address before adding the acocunt
// 2. otherwise, simply add the derived account
export async function lookUpAndAddAccount({
  addAccount,
  aptosAccount,
  aptosClient,
  mnemonic,
}: LookUpAndAddAccountProps) {
  // Look up original address if account key has been rotated previously
  try {
    const newAccount = await lookupOriginalAddress(aptosClient, aptosAccount, mnemonic);
    await addAccount(newAccount);
  } catch (err) {
    // if account failed to be looked up then account key has not been rotated
    // therefore add the account derived from private key or mnemonic string
    // errorCode 'table_item_not_found' means address cannot be found in the table
    if (err instanceof ApiError && err.errorCode === 'table_item_not_found') {
      // eslint-disable-next-line no-console
      console.error('failed to fetch rotated key for address ', aptosAccount.address());

      const newAccount = mnemonic ? {
        mnemonic,
        ...keysFromAptosAccount(aptosAccount),
      } : keysFromAptosAccount(aptosAccount);

      await addAccount(newAccount);
    } else {
      // throw err here so we can catch it later in Import Account component
      // and raise error toast/trigger analytic event
      throw err;
    }
  }
}

export default {
  lookUpAndAddAccount,
  lookUpAndInitAccounts,
  lookupOriginalAddress,
};
