// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  Center,
  Spinner,
  Text,
  useColorMode,
  useRadio,
  UseRadioProps,
} from '@chakra-ui/react';
import { secondaryHoverBgColor, secondaryButtonColor } from 'core/colors';
import { Network, NetworkType } from 'core/hooks/useGlobalState';

type NetworkListItemProps = UseRadioProps & {
  isLoading: boolean,
  network: Network,
};

export default function NetworkListItem(props: NetworkListItemProps) {
  const { getCheckboxProps, getInputProps } = useRadio(props);
  const { colorMode } = useColorMode();
  const {
    isChecked, isDisabled, isLoading, network, value,
  } = props;
  return (
    <Box as="label">
      <input disabled={isDisabled} {...getInputProps()} />
      <Box
        {...getCheckboxProps()}
        cursor="pointer"
        borderRadius="md"
        bgColor={secondaryButtonColor[colorMode]}
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
          !isLoading ? (
            <>
              <Text fontSize="md" fontWeight={600}>
                { network.name }
              </Text>
              <Text fontSize="md" fontWeight={400}>
                { network.nodeUrl }
              </Text>
              {
                (isDisabled && value === NetworkType.LocalHost) ? (
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
