// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
} from '@chakra-ui/react';
import React from 'react';
import { IoIosSend } from '@react-icons/all-files/io/IoIosSend';
import {
  useCoinTransferTransaction,
} from 'core/mutations/transaction';
import { transferAptFormId, TransferFlowProvider, useTransferFlow } from 'core/hooks/useTransferFlow';
import { SubmitHandler } from 'react-hook-form';
import { coinTransferAbortToast, coinTransferSuccessToast, transactionErrorToast } from './Toast';
import TransferDrawer from './TransferDrawer';

function TransferButton() {
  const { coinBalanceOcta, openDrawer } = useTransferFlow();
  return (
    <Button
      disabled={!coinBalanceOcta}
      leftIcon={<IoIosSend />}
      onClick={openDrawer}
      backgroundColor="whiteAlpha.200"
      _hover={{
        backgroundColor: 'whiteAlpha.300',
      }}
      _active={{
        backgroundColor: 'whiteAlpha.400',
      }}
      color="white"
    >
      Send
    </Button>
  );
}

export interface CoinTransferFormData {
  amount?: string;
  recipient?: string;
}

function TransferFlow() {
  const {
    amountAptNumber,
    amountOctaNumber,
    backOnClick,
    canSubmitForm,
    closeDrawer,
    doesRecipientAccountExist,
    formMethods,
    validRecipientAddress,
  } = useTransferFlow();

  const { handleSubmit, reset: resetForm } = formMethods;

  const {
    mutateAsync: submitCoinTransfer,
  } = useCoinTransferTransaction();

  const onSubmit: SubmitHandler<CoinTransferFormData> = async (data, event) => {
    event?.preventDefault();
    if (!canSubmitForm) {
      return;
    }

    try {
      const onChainTxn = await submitCoinTransfer({
        amount: amountOctaNumber,
        doesRecipientExist: doesRecipientAccountExist!,
        recipient: validRecipientAddress!,
      });

      if (onChainTxn.success) {
        coinTransferSuccessToast(amountAptNumber, onChainTxn);
        resetForm();
        backOnClick();
        closeDrawer();
      } else {
        coinTransferAbortToast(onChainTxn);
      }
    } catch (err) {
      transactionErrorToast(err);
    }
  };

  return (
    <>
      <TransferButton />
      <form id={transferAptFormId} onSubmit={handleSubmit(onSubmit)}>
        <TransferDrawer />
      </form>
    </>
  );
}

export default function TransferFlowWrapper() {
  return (
    <TransferFlowProvider>
      <TransferFlow />
    </TransferFlowProvider>
  );
}
