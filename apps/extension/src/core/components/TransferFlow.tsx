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
import useDebounce from 'core/hooks/useDebounce';
import { OCTA_POSITIVE_EXPONENT } from 'core/utils/coin';
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
      colorScheme="teal"
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
    backOnClick,
    canSubmitForm,
    closeDrawer,
    doesRecipientAccountExist,
    formMethods,
    validRecipientAddress,
  } = useTransferFlow();

  const { handleSubmit, reset: resetForm, watch } = formMethods;

  const amount = watch('amount');
  const numberAmountApt = parseFloat(amount || '0');
  const numberAmountOcta = parseInt((numberAmountApt * OCTA_POSITIVE_EXPONENT).toString(), 10);
  const {
    debouncedValue: debouncedNumberAmountOcta,
  } = useDebounce(numberAmountOcta, 500);

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
        amount: debouncedNumberAmountOcta,
        doesRecipientExist: doesRecipientAccountExist!,
        recipient: validRecipientAddress!,
      });

      if (onChainTxn.success) {
        coinTransferSuccessToast(debouncedNumberAmountOcta, onChainTxn);
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
