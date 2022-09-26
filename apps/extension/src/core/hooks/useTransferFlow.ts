// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useDisclosure } from '@chakra-ui/react';
import constate from 'constate';
import { CoinTransferFormData } from 'core/components/TransferFlow';
import { useCoinTransferSimulation } from 'core/mutations/transaction';
import { useAccountExists, useAccountOctaCoinBalance } from 'core/queries/account';
import { formatAddress, isAddressValid } from 'core/utils/address';
import { bigIntMin } from 'core/utils/bigint';
import { formatCoin, OCTA_NUMBER } from 'core/utils/coin';
import { useCallback, useMemo, useState } from 'react';
import { useForm } from 'react-hook-form';
import { parseMoveAbortDetails } from 'shared/move';
import { useActiveAccount } from './useAccounts';
import useDebounce from './useDebounce';

export const transferAptFormId = 'transferApt' as const;

export enum TransferDrawerPage {
  ADD_ADDRESS_AND_AMOUNT = 'Add an address and amount',
  CONFIRM_TRANSACTION = 'Confirm transaction',
}

export const [TransferFlowProvider, useTransferFlow] = constate(() => {
  // hooks
  const formMethods = useForm<CoinTransferFormData>();
  const {
    isOpen: isDrawerOpen,
    onClose: closeDrawer,
    onOpen: openDrawer,
  } = useDisclosure();

  const { activeAccountAddress } = useActiveAccount();
  const { data: coinBalanceOcta } = useAccountOctaCoinBalance(activeAccountAddress);
  const coinBalanceApt = useMemo(() => formatCoin(coinBalanceOcta), [coinBalanceOcta]);

  const [
    transferDrawerPage,
    setTransferDrawerPage,
  ] = useState<TransferDrawerPage>(TransferDrawerPage.ADD_ADDRESS_AND_AMOUNT);

  // form data and methods
  const { watch } = formMethods;
  const amount = watch('amount');
  const recipient = watch('recipient');

  const validRecipientAddress = isAddressValid(recipient) ? formatAddress(recipient) : undefined;
  const { data: doesRecipientAccountExist } = useAccountExists({
    address: validRecipientAddress,
  });

  // todo: this could fail if the user passes a large enough amount to overflow number
  const amountApt = amount?.replace(/[^0-9.]/g, '') ?? '0';
  const aptIntegral = amountApt.split('.')[0];
  const aptFractional = amountApt.split('.')[1] ?? '';
  const amountOcta = BigInt(`${aptIntegral}${aptFractional.padEnd(OCTA_NUMBER, '0')}`);

  const {
    debouncedValue: debouncedNumberAmountOcta,
    isLoading: debouncedAmountIsLoading,
  } = useDebounce(amountOcta, 500);
  const isBalanceEnoughBeforeFee = (debouncedNumberAmountOcta && coinBalanceOcta !== undefined)
    ? debouncedNumberAmountOcta <= coinBalanceOcta
    : undefined;

  const maxGas = coinBalanceOcta
    ? Number(bigIntMin(coinBalanceOcta, BigInt(Number.MAX_SAFE_INTEGER)))
    : 0;
  const {
    data: simulationResult,
    error: simulationError,
  } = useCoinTransferSimulation({
    doesRecipientExist: doesRecipientAccountExist,
    octaAmount: debouncedNumberAmountOcta,
    recipient: validRecipientAddress,
  }, {
    enabled: isDrawerOpen && isBalanceEnoughBeforeFee,
    maxGasOctaAmount: maxGas,
    refetchInterval: 5000,
  });

  const estimatedGasFeeOcta = debouncedNumberAmountOcta === 0n
    ? 0
    : (simulationResult
      && Number(simulationResult.gas_used) * Number(simulationResult.gas_unit_price));

  const estimatedGasFeeApt = useMemo(
    () => formatCoin(estimatedGasFeeOcta, { decimals: 8 }),
    [estimatedGasFeeOcta],
  );

  const simulationAbortDetails = simulationResult !== undefined
    ? parseMoveAbortDetails(simulationResult.vm_status)
    : undefined;

  const shouldBalanceShake = isBalanceEnoughBeforeFee === false
  || simulationError !== null
  || simulationAbortDetails !== undefined;

  const canSubmitForm = validRecipientAddress !== undefined
    && !debouncedAmountIsLoading
    && doesRecipientAccountExist !== undefined
    && debouncedNumberAmountOcta !== undefined
    && debouncedNumberAmountOcta >= 0
    && simulationResult?.success === true
    && !simulationError;

  // transfer page state

  const nextOnClick = useCallback(() => {
    setTransferDrawerPage(TransferDrawerPage.CONFIRM_TRANSACTION);
  }, []);

  const backOnClick = useCallback(() => {
    setTransferDrawerPage(TransferDrawerPage.ADD_ADDRESS_AND_AMOUNT);
  }, []);

  return {
    amountApt,
    amountOcta,
    backOnClick,
    canSubmitForm,
    closeDrawer,
    coinBalanceApt,
    coinBalanceOcta,
    doesRecipientAccountExist,
    estimatedGasFeeApt,
    estimatedGasFeeOcta,
    formMethods,
    isDrawerOpen,
    nextOnClick,
    openDrawer,
    shouldBalanceShake,
    simulationResult,
    transferDrawerPage,
    validRecipientAddress,
  };
});
