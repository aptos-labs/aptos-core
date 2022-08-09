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
  Text,
  useColorMode,
  useDisclosure,
  useRadio,
  useRadioGroup,
  UseRadioProps,
  VStack,
} from '@chakra-ui/react';
import { secondaryTextColor, secondaryBgColor } from 'core/colors';
import { useWalletState } from 'core/hooks/useWalletState';
import {
  useAccountCoinBalance,
} from 'core/queries/account';
import { useAccountLatestTransactionTimestamp } from 'core/queries/transaction';
import Routes from 'core/routes';
import numeral from 'numeral';
import React, { useMemo } from 'react';
import { useNavigate } from 'react-router-dom';

const secondaryHoverBgColor = {
  dark: 'teal.600',
  light: 'gray.200',
};

interface WalletDrawerBodyItemProps {
  address: string;
}

function WalletDrawerBodyListItem(
  props: UseRadioProps & WalletDrawerBodyItemProps,
) {
  const { getCheckboxProps, getInputProps } = useRadio(props);
  const {
    address,
    isChecked,
  } = props;
  const input = getInputProps();
  const checkbox = getCheckboxProps();
  const { colorMode } = useColorMode();
  const navigate = useNavigate();
  const { removeAccount } = useWalletState();
  const { isOpen, onClose, onOpen } = useDisclosure();
  const {
    data: latestTransactionTimestamp,
  } = useAccountLatestTransactionTimestamp({
    address,
    refetchInterval: 4000,
  });

  const { data: coinBalance } = useAccountCoinBalance({ address, refetchInterval: 4000 });
  const coinBalanceString = numeral(coinBalance).format('0,0');

  const accountAddressFormatted = `Account: ${address.substring(0, 15)}...`;

  return useMemo(() => {
    const deleteOnClick = () => {
      // prompt password
      removeAccount({ accountAddress: address });
      onClose();
      navigate(Routes.login.routePath);
    };

    return (
      <Box as="label" width="100%">
        <input {...input} />
        <Box
          {...checkbox}
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
                {accountAddressFormatted}
              </Heading>
            </VStack>
            <DeleteIcon
              fontSize="lg"
              cursor="pointer"
              onClick={onOpen}
              _hover={{
                color: 'red.400',
              }}
            />
            <Modal isOpen={isOpen} onClose={onClose}>
              <ModalOverlay />
              <ModalContent>
                <ModalHeader>
                  Are you sure you want to delete this wallet with address
                  {' '}
                  {address}
                  ?
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
                  <Button colorScheme="red" mr={3} onClick={deleteOnClick}>
                    Yes, I understand
                  </Button>
                  <Button onClick={onClose}>
                    Close
                  </Button>
                </ModalFooter>
              </ModalContent>
            </Modal>
          </Grid>
          <Grid templateColumns="1fr" borderRadius=".5rem" padding={4} pt={2}>
            <HStack
              color={isChecked ? 'gray.300' : secondaryTextColor[colorMode]}
              divider={<span>&nbsp;&nbsp;&bull;&nbsp;&nbsp;</span>}
            >
              <Text noOfLines={1} fontSize="sm">
                Balance:
                {' '}
                {coinBalanceString}
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
    );
  }, [input, checkbox, colorMode, isChecked, accountAddressFormatted,
    onOpen, isOpen, onClose, address, coinBalanceString,
    latestTransactionTimestamp, removeAccount, navigate]);
}

function WalletDrawerBody() {
  const { switchAccount, walletState } = useWalletState();
  const navigate = useNavigate();

  const onClick = (address: string) => {
    if (walletState.accounts) {
      switchAccount({ accountAddress: address });
      navigate(Routes.login.routePath);
    }
  };

  const { getRadioProps, getRootProps } = useRadioGroup({
    defaultValue: walletState.currAccountAddress || undefined,
    name: 'aptosWalletAccount',
    onChange: onClick,
  });

  const group = getRootProps();

  return (
    <VStack {...group} spacing={2} width="100%" py={2}>
      {walletState.accounts && Object.keys(walletState.accounts).map((address) => {
        const radio = getRadioProps({ value: address });
        return (
          <WalletDrawerBodyListItem
            key={address}
            address={address}
            {...radio}
          />
        );
      })}
    </VStack>
  );
}

export default WalletDrawerBody;
