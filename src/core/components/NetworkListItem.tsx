// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Center,
  Spinner,
  Text,
  useColorMode,
  useRadio,
  UseRadioProps,
} from '@chakra-ui/react';
import { LOCAL_NODE_URL } from 'core/constants';
import {
  AptosNetwork,
  networkUriMap,
} from 'core/utils/network';
import React from 'react';

export interface SettingsListItemProps {
  title?: 'Mainnet' | 'Testnet' | 'Devnet' | 'Localhost';
  value: AptosNetwork;
}

const secondaryBgColor = {
  dark: 'gray.600',
  light: 'gray.100',
};

const secondaryHoverBgColor = {
  dark: 'gray.700',
  light: 'gray.200',
};

export default function NetworkListItem(props: UseRadioProps & { isLoading: boolean }) {
  const { getCheckboxProps, getInputProps } = useRadio(props);
  const { colorMode } = useColorMode();
  const {
    isChecked, isDisabled, isLoading, value,
  } = props;
  const input = getInputProps();
  const checkbox = getCheckboxProps();
  return (
    <Box as="label">
      <input disabled={isDisabled && (value === LOCAL_NODE_URL)} {...input} />
      <Box
        {...checkbox}
        cursor="pointer"
        borderRadius="md"
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
        px={5}
        py={3}
      >
        {
          isLoading ? (
            <>
              <Text fontSize="md" fontWeight={600}>
                {value ? networkUriMap[value] : undefined}
              </Text>
              <Text fontSize="md" fontWeight={400}>
                {value}
              </Text>
              {
                (isDisabled && value === LOCAL_NODE_URL) ? (
                  <Text fontSize="sm">(Please start testnet and testnet faucet on localhost to switch)</Text>
                ) : undefined
              }
            </>
          ) : (
            <Center>
              <Spinner />
            </Center>
          )
        }
      </Box>
    </Box>
  );
}
