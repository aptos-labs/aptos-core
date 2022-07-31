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
import { secondaryHoverBgColor, secondaryButtonColor } from 'core/colors';
import {
  NodeUrl,
  NetworkType,
  nodeUrlMap,
  nodeUrlReverseMap,
} from 'core/utils/network';
import React from 'react';

export interface SettingsListItemProps {
  title?: NetworkType;
  value: NodeUrl;
}

export default function NetworkListItem(
  props: UseRadioProps & { isLoading: boolean, value: NodeUrl },
) {
  const { getCheckboxProps, getInputProps } = useRadio(props);
  const { colorMode } = useColorMode();
  const {
    isChecked, isDisabled, isLoading, value,
  } = props;
  const input = getInputProps();
  const checkbox = getCheckboxProps();
  return (
    <Box as="label">
      <input disabled={isDisabled && (value === nodeUrlMap.Localhost)} {...input} />
      <Box
        {...checkbox}
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
          isLoading ? (
            <>
              <Text fontSize="md" fontWeight={600}>
                {value ? nodeUrlReverseMap[value] : undefined}
              </Text>
              <Text fontSize="md" fontWeight={400}>
                {value}
              </Text>
              {
                (isDisabled && value === nodeUrlMap.Localhost) ? (
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
