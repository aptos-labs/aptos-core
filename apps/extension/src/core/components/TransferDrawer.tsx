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
import { FormProvider, useForm } from 'react-hook-form';
import React, { useState } from 'react';
import { IoIosSend } from '@react-icons/all-files/io/IoIosSend';
import {
  useAccountCoinBalance,
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
import numeral from 'numeral';
import useDebounce from 'core/hooks/useDebounce';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { formatAddress, isAddressValid } from 'core/utils/address';
import TransferInput from './TransferInput';
import TransferAvatar from './TransferAvatar';
import TransferSummary from './TransferSummary';

enum TransferDrawerPage {
  ADD_ADDRESS_AND_AMOUNT = 'Add an address and amount',
  CONFIRM_TRANSACTION = 'Confirm transaction',
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
  const {
    data: doesRecipientAccountExist,
  } = useAccountExists({ address: validRecipientAddress });
  const validRecipient = doesRecipientAccountExist ? recipient : undefined;

  const amount = watch('amount');
  const numberAmount = numeral(amount).value() || undefined;
  const {
    debouncedValue: debouncedAmount,
    isLoading: debouncedAmountIsLoading,
  } = useDebounce(numberAmount, 500);
  const { activeAccountAddress } = useActiveAccount();
  const { data: coinBalance } = useAccountCoinBalance(activeAccountAddress);

  const {
    data: simulatedTxn,
  } = useCoinTransferSimulation({
    amount: debouncedAmount,
    create: !doesRecipientAccountExist,
    enabled: isDrawerOpen,
    recipient: validRecipientAddress,
  });

  const {
    isLoading: transferIsLoading,
    isReady: canSubmitTransaction,
    mutateAsync: submitCoinTransfer,
  } = useCoinTransferTransaction();

  const onSubmit = async () => {
    if (!validRecipientAddress || !debouncedAmount) {
      return;
    }
    const onChainTxn = await submitCoinTransfer({
      amount: debouncedAmount,
      create: !doesRecipientAccountExist,
      recipient: validRecipientAddress,
    });
    if (onChainTxn && onChainTxn.success) {
      resetForm();
      setDrawerPage(TransferDrawerPage.ADD_ADDRESS_AND_AMOUNT);
      closeDrawer();
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

  const explorerAddress = `https://explorer.devnet.aptos.dev/account/${recipient}`;
  const estimatedGasFee = debouncedAmount && simulatedTxn && Number(simulatedTxn.gas_used);
  const maxAmount = coinBalance && estimatedGasFee && coinBalance - estimatedGasFee;
  const isBalanceEnough = !maxAmount || debouncedAmount <= maxAmount;

  const shouldBalanceShake = (!isBalanceEnough);

  const canSubmitForm = canSubmitTransaction
    && !debouncedAmountIsLoading
    && validRecipientAddress
    && debouncedAmount
    && isBalanceEnough
    && (!doesRecipientAccountExist || simulatedTxn?.success);

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
                  validRecipient
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
          coinBalance={coinBalance}
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
          amount={numberAmount}
          estimatedGasFee={estimatedGasFee}
          recipient={validRecipientAddress}
          unit="APT"
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
            isLoading={isSubmitting || transferIsLoading}
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
