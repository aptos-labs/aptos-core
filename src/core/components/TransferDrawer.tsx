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
  InputRightAddon,
  SimpleGrid,
  Tag,
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
} from 'core/queries/account';
import { useSubmitTestCoinTransfer } from 'core/mutations/transaction';
import { ExternalLinkIcon } from '@chakra-ui/icons';
import {
  secondaryErrorMessageColor, STATIC_GAS_AMOUNT,
} from 'core/constants';
import numeral from 'numeral';
import { GraceHopperBoringAvatar } from 'core/components/BoringAvatar';
import { secondaryTextColor } from '../../pages/Login';

export const secondaryDividerColor = {
  dark: 'whiteAlpha.300',
  light: 'gray.200',
};

function TransferDrawer() {
  const { colorMode } = useColorMode();
  const { aptosAccount, aptosNetwork } = useWalletState();
  const {
    formState: { errors }, handleSubmit, register, setError, watch,
  } = useForm();
  const addressInputRef = useRef<HTMLInputElement>();
  const { isOpen, onClose, onOpen } = useDisclosure();
  const {
    isLoading: isTransferLoading,
    mutateAsync: submitSendTransaction,
  } = useSubmitTestCoinTransfer();

  const transferAmount: string | undefined | null = watch('transferAmount');
  const transferAmountNumeral = numeral(transferAmount).format('0,0');

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
      const tokenBalance = getTestCoinTokenBalanceFromAccountResources({
        accountResources: fromAccountResources,
      });
      if (Number(transferAmount) >= Number(tokenBalance) - STATIC_GAS_AMOUNT) {
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
                  <Text
                    fontSize="md"
                    noOfLines={1}
                    maxW="280px"
                  >
                    {toAddressStatus}
                  </Text>
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
            <DrawerBody px={0}>
              <VStack spacing={4}>
                <VStack
                  py={10}
                >
                  <Text color={secondaryTextColor[colorMode]}>Send</Text>
                  <Text
                    fontSize="5xl"
                    fontWeight={600}
                    noOfLines={1}
                    maxW="250px"
                  >
                    {transferAmountNumeral}
                  </Text>
                  <Tag
                    borderRadius="full"
                    variant="solid"
                  >
                    coins
                  </Tag>
                </VStack>
                <VStack
                  borderTopWidth="1px"
                  borderTopColor={secondaryDividerColor[colorMode]}
                  pt={4}
                  px={4}
                  width="100%"
                >
                  <InputGroup>
                    <Input
                      variant="filled"
                      placeholder="To address"
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
                  <InputGroup>
                    <Input
                      type="number"
                      variant="filled"
                      placeholder="Transfer amount"
                      min={0}
                      required
                      {...register('transferAmount')}
                    />
                    <InputRightAddon>
                      coins
                    </InputRightAddon>
                  </InputGroup>
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
              <SimpleGrid spacing={4} width="100%" columns={2}>
                <Button colorScheme="teal" isLoading={isTransferLoading} isDisabled={isTransferLoading} type="submit">
                  Submit
                </Button>
                <Button onClick={onClose} isDisabled={isTransferLoading}>
                  Cancel
                </Button>
              </SimpleGrid>
            </DrawerFooter>
          </DrawerContent>
        </form>
      </Drawer>
    </>
  );
}

export default TransferDrawer;
