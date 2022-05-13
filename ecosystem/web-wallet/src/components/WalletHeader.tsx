// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Center, HStack, Text, Tooltip, useClipboard, useColorMode } from '@chakra-ui/react'
import React from 'react'
import useWalletState from '../hooks/useWalletState'

const secondaryHeaderBgColor = {
  light: 'gray.200',
  dark: 'gray.700'
}

export const seconaryAddressFontColor = {
  light: 'gray.500',
  dark: 'gray.400'
}

export default function WalletHeader () {
  const { aptosAccount } = useWalletState()
  const { colorMode } = useColorMode()
  const { onCopy, hasCopied } = useClipboard(
    aptosAccount?.address().hex() || ''
  )

  return (
    <Center
      maxW="100%"
      width="100%"
      py={2}
      bgColor={secondaryHeaderBgColor[colorMode]}
    >
      <HStack px={2}>
        <Text
          fontSize="xs"
          color={seconaryAddressFontColor[colorMode]}
        >
          Address
        </Text>
        <Tooltip label={hasCopied ? 'Copied!' : 'Copy address'} closeDelay={300}>
          <Text whiteSpace="nowrap" as="span">
            <Text
              fontSize="xs"
              as="span"
              whiteSpace="nowrap"
              overflow="hidden"
              display="block"
              noOfLines={1}
              maxW={['100px', '120px']}
              cursor="pointer"
              onClick={onCopy}
            >
              {aptosAccount?.address().hex()}
            </Text>
          </Text>
        </Tooltip>
      </HStack>
    </Center>
  )
}
