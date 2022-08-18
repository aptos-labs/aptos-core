// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { DeleteIcon } from '@chakra-ui/icons';
import {
  Box,
  Button,
  Grid,
  Heading,
  HStack,
  Modal,
  ModalBody,
  ModalCloseButton,
  ModalContent,
  ModalFooter,
  ModalHeader,
  ModalOverlay,
  ModalProps,
  Spinner,
  Text,
  useColorMode,
  useDisclosure,
  useRadio,
  UseRadioProps,
  VStack,
} from '@chakra-ui/react';
import { secondaryTextColor, secondaryBgColor } from 'core/colors';
import {
  useAccountCoinBalance,
} from 'core/queries/account';
import { useAccountLatestTransactionTimestamp } from 'core/queries/transaction';
import numeral from 'numeral';
import React, { useMemo } from 'react';
import { Account } from 'core/types/stateTypes';

const secondaryHoverBgColor = {
  dark: 'teal.600',
  light: 'gray.200',
};

type ConfirmationModalProps = Omit<ModalProps, 'children'> & {
  address: string,
  onConfirm: () => void,
};

function ConfirmationModal(props: ConfirmationModalProps) {
  const { address, onClose, onConfirm } = props;

  return (
    <Modal {...props}>
      <ModalOverlay />
      <ModalContent>
        <ModalHeader>
          {`Are you sure you want to delete this wallet with address ${address}?`}
        </ModalHeader>
        <ModalCloseButton />
        <ModalBody>
          <Text fontSize="md">
            PLEASE NOTE: You will not be able to recover this
            account unless you have stored the
            private key or mnemonic associated with
            this wallet address.
          </Text>
        </ModalBody>
        <ModalFooter>
          <Button colorScheme="red" mr={3} onClick={onConfirm}>
            Yes, I understand
          </Button>
          <Button onClick={onClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}

interface AccountDrawerItemProps {
  account: Account;
  onRemove: (address: string) => void;
}

function AccountDrawerItem(
  props: UseRadioProps & AccountDrawerItemProps,
) {
  const { getCheckboxProps, getInputProps } = useRadio(props);
  const { account: { address, name }, isChecked, onRemove } = props;
  const { colorMode } = useColorMode();
  const { isOpen, onClose, onOpen } = useDisclosure();

  const {
    data: latestTransactionTimestamp,
  } = useAccountLatestTransactionTimestamp(address, {
    refetchInterval: 4000,
  });
  const {
    data: coinBalance,
    isLoading: isBalanceLoading,
  } = useAccountCoinBalance(address, {
    refetchInterval: 4000,
  });
  const coinBalanceString = (coinBalance && coinBalance > 1e6)
    ? `~${numeral(coinBalance).format('0.0a')}`
    : numeral(coinBalance).format('0,0');

  const walletAddressFormatted = `${name}: ${address.substring(0, 15)}...`;

  return useMemo(() => (
    <Box as="label" width="100%">
      <input {...getInputProps()} />
      <Box
        {...getCheckboxProps()}
        cursor="pointer"
        bgColor={secondaryBgColor[colorMode]}
        _checked={{
          bg: 'teal.600',
          color: 'white',
        }}
        _hover={{
          bg: (isChecked) ? 'teal.700' : secondaryHoverBgColor[colorMode],
        }}
        _focus={{
          boxShadow: 'outline',
        }}
        borderRadius="md"
      >
        <Grid
          templateColumns="1fr 18px"
          borderRadius=".5rem"
          paddingTop={4}
          paddingX={4}
          cursor="pointer"
        >
          <VStack alignItems="flex-start">
            <Heading fontSize="lg" fontWeight={500} noOfLines={1} maxW={80}>
              {walletAddressFormatted}
            </Heading>
          </VStack>
          <DeleteIcon
            fontSize="lg"
            cursor="pointer"
            onClick={(e: React.MouseEvent) => {
              e.preventDefault();
              onOpen();
            }}
            _hover={{
              color: 'red.400',
            }}
          />
          <ConfirmationModal
            isOpen={isOpen}
            onClose={onClose}
            onConfirm={() => onRemove(address)}
            address={address}
          />
        </Grid>
        <Grid templateColumns="1fr" borderRadius=".5rem" padding={4} pt={2}>
          <HStack
            color={isChecked ? 'gray.300' : secondaryTextColor[colorMode]}
            divider={<span>&nbsp;&nbsp;&bull;&nbsp;&nbsp;</span>}
          >
            <Text noOfLines={1} fontSize="sm">
              Balance:
              &nbsp;
              { isBalanceLoading ? <Spinner size="xs" thickness="1px" as="span" /> : coinBalanceString }
              &nbsp;
              APT
            </Text>
            <Text noOfLines={1} fontSize="sm">
              {
                (latestTransactionTimestamp?.toDateString())
                  ? `Last txn: ${latestTransactionTimestamp?.toDateString()}`
                  : 'No transactions'
              }
            </Text>
          </HStack>
        </Grid>
      </Box>
    </Box>
  ), [getInputProps, getCheckboxProps, colorMode, isChecked,
    walletAddressFormatted, isOpen, onClose, address, isBalanceLoading,
    coinBalanceString, latestTransactionTimestamp, onOpen, onRemove]);
}

export default AccountDrawerItem;
