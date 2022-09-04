// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { createStandaloneToast } from '@chakra-ui/react';
import { UserTransaction } from 'aptos/dist/generated';
import { parseMoveAbortDetails } from 'shared/move';

export const { toast } = createStandaloneToast({
  defaultOptions: {
    duration: 2000,
    isClosable: true,
    variant: 'solid',
  },
});

// Add Account
export const createAccountToast = () => {
  toast({
    description: 'Successfully created new account',
    status: 'success',
    title: 'Created account',
  });
};

export const createAccountErrorToast = () => {
  toast({
    description: 'Error creating new account',
    status: 'error',
    title: 'Error creating account',
  });
};

export const importAccountToast = () => {
  toast({
    description: 'Successfully imported new account',
    status: 'success',
    title: 'Imported account',
  });
};

export const importAccountErrorToast = () => {
  toast({
    description: 'Error importing new account',
    status: 'error',
    title: 'Error importing account',
  });
};

export const importAccountErrorAccountAlreadyExistsToast = () => {
  toast({
    description: 'Account already exists in wallet',
    status: 'error',
    title: 'Error importing account',
  });
};

export const importAccountNotFoundToast = () => {
  toast({
    description: 'Account does not exist on-chain (please note devnet is wiped every 2 weeks)',
    status: 'error',
    title: 'Error importing account',
  });
};

// Switch Account

export const switchAccountToast = (accountAddress: string) => {
  toast({
    description: `Successfully switched account to ${accountAddress.substring(0, 6)}...`,
    status: 'success',
    title: 'Switched account',
  });
};

export const switchAccountErrorToast = () => {
  toast({
    description: 'Error during account switch',
    status: 'error',
    title: 'Error switch account',
  });
};

// Change Password

export const changePasswordNewPasswordNotMatchErrorToast = () => {
  toast({
    description: "New password and confirmed new password don't match",
    status: 'error',
    title: 'Passsword do not match',
  });
};

export const changePasswordIncorrectCurrentPasswordErrorToast = () => {
  toast({
    description: 'Current password entered is incorrect',
    status: 'error',
    title: 'Incorrect current password',
  });
};

export const changePasswordSuccessfullyUpdatedToast = () => {
  toast({
    description: 'Password successfully updated to new password',
    status: 'success',
    title: 'Password updated',
  });
};

// Remove Account

export const removeAccountToast = (message: string) => {
  toast({
    description: message,
    status: 'success',
    title: 'Deleted account',
  });
};

export const removeAccountErrorToast = () => {
  toast({
    description: 'Account deletion process incurred an error',
    status: 'error',
    title: 'Error deleting account',
  });
};

export const addNetworkToast = (networkName?: string) => {
  const description = networkName
    ? `Switching to ${networkName}`
    : 'Staying on current network';
  toast({
    description,
    status: 'success',
    title: 'Added network',
  });
};

export const switchNetworkToast = (networkName: string, isSwitching: boolean) => {
  const description = isSwitching
    ? `Switching to ${networkName}`
    : `Staying on ${networkName}`;
  toast({
    description,
    status: 'success',
    title: 'Removed network',
  });
};

export const networkDoesNotExistToast = () => {
  toast({
    description: 'Error: network not found',
    status: 'error',
    title: 'Error getting network',
  });
};

// transfer

export function coinTransferSuccessToast(amount: number, txn: UserTransaction) {
  toast({
    description: `Amount transferred: ${amount}, gas consumed: ${txn.gas_used}`,
    status: 'success',
    title: 'Transaction succeeded',
  });
}

export function coinTransferAbortToast(txn: UserTransaction) {
  const abortDetails = parseMoveAbortDetails(txn.vm_status);
  const abortReasonDescr = abortDetails !== undefined
    ? abortDetails.reasonDescr
    : 'Transaction failed';
  toast({
    description: `${abortReasonDescr}, gas consumed: ${txn.gas_used}`,
    status: 'error',
    title: 'Transaction failed',
  });
}

export function transactionErrorToast(err: unknown) {
  const errorMsg = err instanceof Error
    ? err.message
    : 'Unexpected error';

  toast({
    description: errorMsg,
    status: 'error',
    title: 'Transaction error',
  });
}

export default toast;
