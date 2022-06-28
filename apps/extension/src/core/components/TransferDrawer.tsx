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
  Input,
  InputGroup,
  SimpleGrid,
  Text,
  useColorMode,
  useDisclosure,
  VStack,
} from '@chakra-ui/react';
import { SubmitHandler, useForm } from 'react-hook-form';
import React, { useMemo, useRef } from 'react';
import { IoIosSend } from 'react-icons/io';
import useWalletState from 'core/hooks/useWalletState';
import {
  getAccountExists,
  getAccountResources,
  getTestCoinTokenBalanceFromAccountResources,
  useAccountExists,
  useAccountResources,
} from 'core/queries/account';
import { useSubmitTestCoinTransfer } from 'core/mutations/transaction';
import { ExternalLinkIcon } from '@chakra-ui/icons';
import {
  secondaryErrorMessageColor, STATIC_GAS_AMOUNT,
} from 'core/constants';
import { GraceHopperBoringAvatar } from 'core/components/BoringAvatar';
import numeral from 'numeral';
import { secondaryTextColor } from '../../pages/Login';

export const secondaryDividerColor = {
  dark: 'whiteAlpha.300',
  light: 'gray.200',
};

interface FormValues {
  toAddress: string;
  transferAmount: string;
}

function TransferDrawer() {
  const { colorMode } = useColorMode();
  const { aptosAccount, aptosNetwork } = useWalletState();
  const {
    formState: { errors }, handleSubmit, register, setError, watch,
  } = useForm<FormValues>();
  const addressInputRef = useRef<HTMLInputElement>();
  const { isOpen, onClose, onOpen } = useDisclosure();
  const {
    isLoading: isTransferLoading,
    mutateAsync: submitSendTransaction,
  } = useSubmitTestCoinTransfer();
  const {
    data: accountResources,
  } = useAccountResources();

  const tokenBalance = getTestCoinTokenBalanceFromAccountResources({ accountResources });
  const tokenBalanceString = numeral(tokenBalance).format('0,0.0000');

  const transferAmount: string | undefined | null = watch('transferAmount');
  const transferAmountNumeral = numeral(transferAmount).format('0,0');
  const transferAmountInputFontSize = useMemo(() => {
    if (!transferAmount) {
      return 64;
    }
    if (transferAmount.length <= 6) {
      return 64;
    }
    if (transferAmount.length > 6 && transferAmount.length <= 10) {
      return 48;
    }
    return 36;
  }, [transferAmount]);

  const {
    onChange: addressOnChange,
    ref: toAddressRef,
    ...toAddressRest
  } = { ...register('toAddress') };
  const toAddress: string | undefined | null = watch('toAddress');
  const explorerAddress = `https://explorer.devnet.aptos.dev/account/${toAddress}`;
  const { data: toAddressAccountExists } = useAccountExists({
    address: toAddress || '',
    debounceTimeout: 5000,
  });

  const onSubmit: SubmitHandler<Record<string, any>> = async (data, event) => {
    event?.preventDefault();
    if (toAddress && aptosAccount && transferAmount) {
      const toAccountExists = await getAccountExists({ address: toAddress, nodeUrl: aptosNetwork });
      if (!toAccountExists) {
        setError('toAddress', { message: 'Invalid account address', type: 'custom' });
        return;
      }
      const fromAccountResources = await getAccountResources({
        address: aptosAccount.address().hex(),
        nodeUrl: aptosNetwork,
      });
      const currentTokenBalance = getTestCoinTokenBalanceFromAccountResources({
        accountResources: fromAccountResources,
      });
      if (Number(transferAmount) >= Number(currentTokenBalance) - STATIC_GAS_AMOUNT) {
        setError('toAddress', { message: 'Insufficient balance', type: 'custom' });
        return;
      }
      await submitSendTransaction({
        amount: transferAmount,
        fromAccount: aptosAccount,
        nodeUrl: aptosNetwork,
        onClose,
        toAddress,
      });
    }
  };

  const toAddressStatus = useMemo(() => {
    if (!toAddress) {
      return 'Please enter an address';
    } if (toAddressAccountExists && toAddress) {
      return toAddress;
    }
    return 'Invalid address';
  }, [toAddressAccountExists, toAddress]);

  return (
    <>
      <Button
        isLoading={isTransferLoading}
        isDisabled={isTransferLoading}
        leftIcon={<IoIosSend />}
        onClick={onOpen}
      >
        Send
      </Button>
      <Drawer
        size="xl"
        onClose={onClose}
        isOpen={isOpen}
        placement="bottom"
        initialFocusRef={(addressInputRef as React.RefObject<HTMLInputElement>)}
      >
        <DrawerOverlay />
        <form onSubmit={handleSubmit(onSubmit)}>
          <DrawerContent>
            <DrawerHeader borderBottomWidth="1px" px={4}>
              <Grid templateColumns="32px 1fr" gap={4} maxW="100%">
                <Box pt={1}>
                  <Box width="32px">
                    <GraceHopperBoringAvatar type={(toAddressAccountExists) ? 'beam' : 'marble'} />
                  </Box>
                </Box>
                <VStack boxSizing="border-box" spacing={0} alignItems="flex-start" width="100%" maxW="100%">
                  <InputGroup pb={1}>
                    <Input
                      fontWeight={600}
                      size="sm"
                      variant="unstyled"
                      placeholder={toAddressStatus}
                      required
                      maxLength={70}
                      minLength={60}
                      onChange={addressOnChange}
                      autoComplete="off"
                      ref={(e) => {
                        toAddressRef(e);
                        addressInputRef.current = e || undefined;
                      }}
                      {...toAddressRest}
                    />
                  </InputGroup>
                  {toAddressAccountExists ? (
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
                    >
                      View on explorer
                    </Button>
                  ) : (
                    <Button
                      color={secondaryTextColor[colorMode]}
                      fontSize="sm"
                      fontWeight={400}
                      height="24px"
                      variant="unstyled"
                      cursor="default"
                    >
                      Account not found
                    </Button>
                  )}
                </VStack>
              </Grid>
            </DrawerHeader>
            <DrawerBody px={0} py={0}>
              <VStack width="100%" spacing={0}>
                <VStack
                  spacing={8}
                  width="100%"
                >
                  <InputGroup>
                    <Input
                      textAlign="center"
                      type="number"
                      variant="filled"
                      placeholder="0"
                      min={0}
                      py={32}
                      fontSize={transferAmountInputFontSize}
                      borderRadius="0px"
                      required
                      size="lg"
                      _focusVisible={{
                        outline: 'none',
                      }}
                      {...register('transferAmount', {
                        max: 10000000,
                      })}
                    />
                  </InputGroup>
                </VStack>
                <VStack
                  borderTopWidth="1px"
                  borderTopColor={secondaryDividerColor[colorMode]}
                  p={4}
                  width="100%"
                  spacing={0}
                  mt={0}
                >
                  <SimpleGrid width="100%" columns={2}>
                    <Flex>
                      <Text fontWeight={600} fontSize="md">
                        Balance
                      </Text>
                    </Flex>
                    <Flex justifyContent="right">
                      <Text color={secondaryTextColor[colorMode]} fontSize="md">
                        {tokenBalanceString}
                        {' '}
                        coins
                      </Text>
                    </Flex>
                  </SimpleGrid>
                  <Flex overflowY="auto" maxH="100px">
                    <Text
                      fontSize="xs"
                      color={secondaryErrorMessageColor[colorMode]}
                      wordBreak="break-word"
                    >
                      {errors?.toAddress?.message}
                    </Text>
                  </Flex>
                </VStack>
              </VStack>
            </DrawerBody>
            <DrawerFooter borderTopColor={secondaryDividerColor[colorMode]} borderTopWidth="1px" px={4}>
              <Grid gap={4} width="100%" templateColumns="2fr 1fr">
                <Button colorScheme="teal" isLoading={isTransferLoading} isDisabled={isTransferLoading} type="submit">
                  Send
                  {' '}
                  {transferAmountNumeral}
                  {' '}
                  coins
                </Button>
                <Button onClick={onClose} isDisabled={isTransferLoading}>
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
