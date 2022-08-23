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
  Flex,
  Grid,
  HStack,
  Input,
  SimpleGrid,
  Spinner,
  Text,
  useColorMode,
  useDisclosure,
  VStack,
} from '@chakra-ui/react';
import { SubmitHandler, useForm } from 'react-hook-form';
import React, { useMemo } from 'react';
import { IoIosSend } from '@react-icons/all-files/io/IoIosSend';
import {
  useAccountCoinBalance,
  useAccountExists,
} from 'core/queries/account';
import {
  useCoinTransferSimulation,
  useCoinTransferTransaction,
} from 'core/mutations/transaction';
import { ExternalLinkIcon } from '@chakra-ui/icons';
import { secondaryDividerColor, secondaryErrorMessageColor, secondaryTextColor } from 'core/colors';
import { GraceHopperBoringAvatar } from 'core/components/BoringAvatar';
import numeral from 'numeral';
import useDebounce from 'core/hooks/useDebounce';
import { useActiveAccount } from 'core/hooks/useAccounts';

interface CoinTransferFormData {
  amount?: number;
  recipient?: string;
}

function isAddressValid(address?: string) {
  return address
    ? (address.length >= 64 && address.length <= 68)
    : false;
}

function formatAddress(address?: string) {
  return (address && address.startsWith('0x')) ? address : `0x${address}`;
}

function getAmountInputFontSize(amount?: number) {
  if (!amount || amount < 1e7) {
    return 64;
  }
  if (amount < 1e11) {
    return 48;
  }
  return 36;
}

function TransferDrawer() {
  const { colorMode } = useColorMode();
  const {
    isOpen: isDrawerOpen,
    onClose: closeDrawer,
    onOpen: openDrawer,
  } = useDisclosure();

  const {
    formState: { isSubmitted, isSubmitting },
    handleSubmit,
    register,
    reset: resetForm,
    watch,
  } = useForm<CoinTransferFormData>();

  const recipient = watch('recipient');
  const validRecipientAddress = isAddressValid(recipient) ? formatAddress(recipient) : undefined;
  const {
    data: doesRecipientAccountExist,
  } = useAccountExists({ address: validRecipientAddress });
  const validRecipient = doesRecipientAccountExist ? recipient : undefined;

  const amount = watch('amount');
  const amountInputFontSize = useMemo(() => getAmountInputFontSize(amount), [amount]);
  const debouncedAmount = useDebounce(amount, 500);
  const amountNumeral = numeral(debouncedAmount).format('0,0');

  const { activeAccountAddress } = useActiveAccount();
  const { data: coinBalance } = useAccountCoinBalance(activeAccountAddress);
  const coinBalanceString = numeral(coinBalance).format('0,0');

  const {
    data: simulatedTxn,
    isFetching: isSimulationLoading,
  } = useCoinTransferSimulation({
    amount: debouncedAmount,
    create: !doesRecipientAccountExist,
    enabled: isDrawerOpen,
    recipient: validRecipientAddress,
  });

  const {
    isReady: canSubmitTransaction,
    mutateAsync: submitCoinTransfer,
  } = useCoinTransferTransaction();

  const onSubmit: SubmitHandler<CoinTransferFormData> = async (data, event) => {
    event?.preventDefault();
    if (!validRecipientAddress || !debouncedAmount) {
      return;
    }
    const onChainTxn = await submitCoinTransfer({
      amount: debouncedAmount,
      create: !doesRecipientAccountExist,
      recipient: validRecipientAddress,
    });
    if (onChainTxn && onChainTxn.success) {
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

  const explorerAddress = `https://explorer.devnet.aptos.dev/account/${recipient}`;
  const estimatedGasFee = debouncedAmount && simulatedTxn && Number(simulatedTxn.gas_used);
  const maxAmount = coinBalance && estimatedGasFee && coinBalance - estimatedGasFee;
  const isBalanceEnough = !maxAmount || debouncedAmount <= maxAmount;

  const canSubmitForm = canSubmitTransaction
    && validRecipientAddress
    && debouncedAmount
    && isBalanceEnough
    && (!doesRecipientAccountExist || simulatedTxn?.success);

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
      <Drawer
        size="xl"
        isOpen={isDrawerOpen}
        onClose={closeDrawer}
        placement="bottom"
        onCloseComplete={onCloseComplete}
      >
        <DrawerOverlay />
        <form onSubmit={handleSubmit(onSubmit)}>
          <DrawerContent>
            <DrawerHeader borderBottomWidth="1px" px={4}>
              <HStack spacing={4}>
                <Box width="32px">
                  <GraceHopperBoringAvatar type={(doesRecipientAccountExist) ? 'beam' : 'marble'} />
                </Box>
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
              <VStack spacing={0}>
                <Input
                  autoComplete="off"
                  textAlign="center"
                  type="number"
                  variant="filled"
                  placeholder="0"
                  py={32}
                  fontSize={amountInputFontSize}
                  borderRadius="0px"
                  size="lg"
                  _focusVisible={{
                    outline: 'none',
                  }}
                  {...register('amount', { valueAsNumber: true })}
                />
                <VStack
                  borderTopWidth="1px"
                  borderTopColor={secondaryDividerColor[colorMode]}
                  p={4}
                  width="100%"
                  spacing={0}
                  mt={0}
                >
                  <SimpleGrid width="100%" columns={2} gap={1}>
                    <Flex>
                      <Text fontWeight={600} fontSize="md">
                        Balance
                      </Text>
                    </Flex>
                    <Flex justifyContent="right">
                      <Text color={secondaryTextColor[colorMode]} fontSize="md">
                        {`${coinBalanceString} coins`}
                      </Text>
                    </Flex>
                    <Flex>
                      <Text fontWeight={600} fontSize="md">
                        Fee
                      </Text>
                    </Flex>
                    <Flex justifyContent="right">
                      <Text color={secondaryTextColor[colorMode]} fontSize="md" as="span">
                        { isSimulationLoading ? (<Spinner size="xs" />) : estimatedGasFee || 0 }
                        { ' coins' }
                      </Text>
                    </Flex>
                  </SimpleGrid>
                  <Flex overflowY="auto" maxH="100px">
                    <Text
                      fontSize="xs"
                      color={secondaryErrorMessageColor[colorMode]}
                      wordBreak="break-word"
                    >
                      { isBalanceEnough || 'Insufficient funds' }
                    </Text>
                  </Flex>
                </VStack>
              </VStack>
            </DrawerBody>
            <DrawerFooter borderTopColor={secondaryDividerColor[colorMode]} borderTopWidth="1px" px={4}>
              <Grid gap={4} width="100%" templateColumns="2fr 1fr">
                <Button
                  colorScheme="teal"
                  isLoading={isSubmitting}
                  isDisabled={!canSubmitForm}
                  type="submit"
                >
                  { `Send ${amountNumeral} coins` }
                </Button>
                <Button onClick={closeDrawer} isDisabled={isSubmitting}>
                  Cancel
                </Button>
              </Grid>
            </DrawerFooter>
          </DrawerContent>
        </form>
      </Drawer>
    </>
  );
}

export default TransferDrawer;
