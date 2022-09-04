// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Button,
  Drawer,
  DrawerBody,
  DrawerContent,
  DrawerFooter,
  DrawerHeader,
  DrawerOverlay,
  Fade,
  Grid,
  HStack,
  Input,
  Text,
  useColorMode,
  useDisclosure,
  VStack,
} from '@chakra-ui/react';
import { Types } from 'aptos';
import { FormProvider, useForm } from 'react-hook-form';
import React, { useState } from 'react';
import { IoIosSend } from '@react-icons/all-files/io/IoIosSend';
import {
  useAccountAptosCoinBalance,
  useAccountExists,
} from 'core/queries/account';
import {
  useCoinTransferSimulation,
  useCoinTransferTransaction,
} from 'core/mutations/transaction';
import {
  ExternalLinkIcon,
} from '@chakra-ui/icons';
import { secondaryDividerColor, secondaryErrorMessageColor, secondaryTextColor } from 'core/colors';
import toast from 'core/components/Toast';
import useDebounce from 'core/hooks/useDebounce';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { formatAddress, isAddressValid } from 'core/utils/address';
import { parseMoveAbortDetails } from 'shared/move';
import { OCTA_POSITIVE_EXPONENT } from 'core/utils/coin';
import TransferInput from './TransferInput';
import TransferAvatar from './TransferAvatar';
import TransferSummary from './TransferSummary';

type UserTransaction = Types.UserTransaction;

enum TransferDrawerPage {
  ADD_ADDRESS_AND_AMOUNT = 'Add an address and amount',
  CONFIRM_TRANSACTION = 'Confirm transaction',
}

function coinTransferSuccessToast(amount: number, txn: UserTransaction) {
  toast({
    description: `Amount transferred: ${amount}, gas consumed: ${txn.gas_used}`,
    status: 'success',
    title: 'Transaction succeeded',
  });
}

function coinTransferAbortToast(txn: UserTransaction) {
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

function transactionErrorToast(err: unknown) {
  const errorMsg = err instanceof Error
    ? err.message
    : 'Unexpected error';

  toast({
    description: errorMsg,
    status: 'error',
    title: 'Transaction error',
  });
}

export interface CoinTransferFormData {
  amount?: string;
  recipient?: string;
}

function TransferDrawer() {
  const { colorMode } = useColorMode();
  const {
    isOpen: isDrawerOpen,
    onClose: closeDrawer,
    onOpen: openDrawer,
  } = useDisclosure();
  const [
    drawerPage,
    setDrawerPage,
  ] = useState<TransferDrawerPage>(
    TransferDrawerPage.ADD_ADDRESS_AND_AMOUNT,
  );

  const formMethods = useForm<CoinTransferFormData>();
  const {
    formState: { isSubmitted, isSubmitting },
    register,
    reset: resetForm,
    watch,
  } = formMethods;

  const recipient = watch('recipient');
  const validRecipientAddress = isAddressValid(recipient) ? formatAddress(recipient) : undefined;
  const { data: doesRecipientAccountExist } = useAccountExists({
    address: validRecipientAddress,
  });

  const amount = watch('amount');
  const numberAmountApt = parseFloat(amount || '0');
  const numberAmountOcta = parseInt((numberAmountApt * OCTA_POSITIVE_EXPONENT).toString(), 10);
  const {
    debouncedValue: debouncedNumberAmountOcta,
    isLoading: debouncedAmountIsLoading,
  } = useDebounce(numberAmountOcta, 500);
  const { activeAccountAddress } = useActiveAccount();
  const { data: coinBalance } = useAccountAptosCoinBalance(activeAccountAddress);
  const isBalanceEnoughBeforeFee = (debouncedNumberAmountOcta && coinBalance !== undefined)
    ? debouncedNumberAmountOcta <= coinBalance?.OCTA
    : undefined;

  const {
    data: simulationResult,
    error: simulationError,
  } = useCoinTransferSimulation({
    doesRecipientExist: doesRecipientAccountExist,
    octaAmount: debouncedNumberAmountOcta,
    recipient: validRecipientAddress,
  }, {
    enabled: isDrawerOpen && isBalanceEnoughBeforeFee,
    maxGasOctaAmount: coinBalance?.OCTA || 0,
    refetchInterval: 5000,
  });

  const estimatedGasFee = debouncedNumberAmountOcta
   && simulationResult
   && Number(simulationResult.gas_used);
  const { mutateAsync: submitCoinTransfer } = useCoinTransferTransaction({ estimatedGasFee });

  const explorerAddress = `https://explorer.devnet.aptos.dev/account/${recipient}`;
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

  console.log(validRecipientAddress);
  console.log(!debouncedAmountIsLoading);
  console.log(doesRecipientAccountExist);
  console.log(debouncedNumberAmountOcta);
  console.log(simulationResult?.success);
  console.log(!simulationError);

  const onSubmit = async () => {
    if (!canSubmitForm) {
      return;
    }

    try {
      const onChainTxn = await submitCoinTransfer({
        amount: debouncedNumberAmountOcta,
        doesRecipientExist: doesRecipientAccountExist,
        recipient: validRecipientAddress,
      });

      if (onChainTxn.success) {
        coinTransferSuccessToast(debouncedNumberAmountOcta, onChainTxn);
        resetForm();
        setDrawerPage(TransferDrawerPage.ADD_ADDRESS_AND_AMOUNT);
        closeDrawer();
      } else {
        coinTransferAbortToast(onChainTxn);
      }
    } catch (err) {
      transactionErrorToast(err);
    }
  };

  // When the drawer is closed, reset the form only if the
  // transfer was successful
  const onCloseComplete = () => {
    if (isSubmitted) {
      resetForm();
    }
  };

  const nextOnClick = () => {
    setDrawerPage(TransferDrawerPage.CONFIRM_TRANSACTION);
  };

  const backOnClick = () => {
    setDrawerPage(TransferDrawerPage.ADD_ADDRESS_AND_AMOUNT);
  };

  const addAddressAndAmountDrawerContent = (
    <>
      <DrawerHeader borderBottomWidth="1px" px={4} position="relative">
        <Box
          position="absolute"
          top="0px"
          width="100%"
        >
          <Text
            fontSize="3xl"
            fontWeight={600}
            position="absolute"
            bottom="1rem"
          >
            {drawerPage}
          </Text>
        </Box>
        <HStack spacing={4}>
          <TransferAvatar
            doesRecipientAccountExist={doesRecipientAccountExist}
            recipient={recipient}
          />
          <VStack boxSizing="border-box" spacing={0} alignItems="flex-start" flexGrow={1}>
            <Input
              pb={1}
              variant="unstyled"
              size="sm"
              fontWeight={600}
              autoComplete="off"
              spellCheck="false"
              placeholder="Please enter an address"
              {...register('recipient')}
            />
            {doesRecipientAccountExist ? (
              <Button
                color={secondaryTextColor[colorMode]}
                fontSize="sm"
                fontWeight={400}
                height="24px"
                as="a"
                target="_blank"
                rightIcon={<ExternalLinkIcon />}
                variant="unstyled"
                cursor="pointer"
                href={explorerAddress}
                tabIndex={-1}
              >
                View on explorer
              </Button>
            ) : (
              <Button
                color={
                  doesRecipientAccountExist
                    ? secondaryTextColor[colorMode]
                    : secondaryErrorMessageColor[colorMode]
                }
                fontSize="sm"
                fontWeight={400}
                height="24px"
                variant="unstyled"
                cursor="default"
              >
                { validRecipientAddress
                  ? 'Account not found, will be created'
                  : 'Invalid address' }
              </Button>
            )}
          </VStack>
        </HStack>
      </DrawerHeader>
      <DrawerBody px={0} py={0}>
        <TransferInput
          estimatedGasFee={estimatedGasFee}
          coinBalance={coinBalance?.APT}
          doesRecipientAccountExist={doesRecipientAccountExist}
          shouldBalanceShake={shouldBalanceShake}
        />
      </DrawerBody>
      <DrawerFooter
        borderTopColor={secondaryDividerColor[colorMode]}
        borderTopWidth="1px"
        px={4}
      >
        <Grid gap={4} width="100%" templateColumns="1fr 1fr">
          <Button onClick={closeDrawer}>
            Cancel
          </Button>
          <Button
            isLoading={isSubmitting}
            isDisabled={!canSubmitForm}
            colorScheme="teal"
            onClick={nextOnClick}
          >
            Next
          </Button>
        </Grid>
      </DrawerFooter>
    </>
  );

  const submitTransactionDrawerContent = (
    <>
      <DrawerHeader borderBottomWidth="1px" px={4} position="relative">
        <Box
          position="absolute"
          top="0px"
          width="100%"
        >
          <Text
            fontSize="3xl"
            fontWeight={600}
            position="absolute"
            bottom="1rem"
          >
            {drawerPage}
          </Text>
        </Box>
        <HStack spacing={4}>
          <Fade in={drawerPage === TransferDrawerPage.CONFIRM_TRANSACTION}>
            <Text>Summary</Text>
          </Fade>
        </HStack>
      </DrawerHeader>
      <DrawerBody px={0} py={0}>
        <TransferSummary
          amount={numberAmountApt}
          estimatedGasFee={estimatedGasFee}
          recipient={validRecipientAddress}
        />
      </DrawerBody>
      <DrawerFooter
        borderTopColor={secondaryDividerColor[colorMode]}
        borderTopWidth="1px"
        px={4}
      >
        <Grid gap={4} width="100%" templateColumns="1fr 1fr">
          <Button onClick={backOnClick}>
            Back
          </Button>
          <Button
            isLoading={isSubmitting}
            isDisabled={!canSubmitForm}
            colorScheme="teal"
            type="submit"
            onClick={onSubmit}
          >
            Send
          </Button>
        </Grid>
      </DrawerFooter>
    </>
  );

  return (
    <>
      <Button
        disabled={!coinBalance}
        leftIcon={<IoIosSend />}
        onClick={openDrawer}
        colorScheme="teal"
      >
        Send
      </Button>
      <FormProvider {...formMethods}>
        <form>
          <Drawer
            size="xl"
            isOpen={isDrawerOpen}
            onClose={closeDrawer}
            placement="bottom"
            onCloseComplete={onCloseComplete}
          >
            <DrawerOverlay bgColor="rgba(57,178,172, 0.4)" backdropFilter="blur(1rem)" />
            <DrawerContent className="drawer-content" borderTopRadius=".5rem">
              {drawerPage === TransferDrawerPage.ADD_ADDRESS_AND_AMOUNT ? (
                addAddressAndAmountDrawerContent
              ) : submitTransactionDrawerContent}
            </DrawerContent>
          </Drawer>
        </form>
      </FormProvider>
    </>
  );
}

export default TransferDrawer;
