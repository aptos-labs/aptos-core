// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { ChevronLeftIcon } from '@chakra-ui/icons'
import { Center, Grid, IconButton, Text, useColorMode } from '@chakra-ui/react'
import React from 'react'
import ChakraLink from './ChakraLink'

const secondaryHeaderBgColor = {
  light: 'gray.200',
  dark: 'gray.700'
}

export const seconaryAddressFontColor = {
  light: 'gray.500',
  dark: 'gray.400'
}

export default function CreateWalletHeader () {
  const { colorMode } = useColorMode()

  return (
    <Center
      maxW="100%"
      width="100%"
      py={2}
      bgColor={secondaryHeaderBgColor[colorMode]}
    >
      <Grid templateColumns="32px 1fr 32px" px={4} width="100%" gap={4}>
        <ChakraLink to="/">
          <IconButton size="xs" borderRadius="full" aria-label="go back" icon={<ChevronLeftIcon fontSize="md" />} />
        </ChakraLink>
        <Text
          fontSize="md"
          textAlign="center"
        >
          Create Wallet
        </Text>
        <></>
      </Grid>
    </Center>
  )
}
