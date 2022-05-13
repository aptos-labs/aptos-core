// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Center, IconButton, SimpleGrid, useColorMode } from '@chakra-ui/react'
import { IoIosPerson, IoMdImage } from 'react-icons/io'
import { RiCopperCoinFill } from 'react-icons/ri'
import React from 'react'
import ChakraLink from './ChakraLink'
import { useLocation } from 'react-router-dom'

const secondaryHeaderBgColor = {
  light: 'gray.200',
  dark: 'gray.700'
}

const secondaryIconColor = {
  light: 'gray.800',
  dark: 'white'
}

export default function WalletFooter () {
  const { colorMode } = useColorMode()
  const { pathname } = useLocation()

  return (
    <Center
      maxW="100%"
      width="100%"
      py={2}
      bgColor={secondaryHeaderBgColor[colorMode]}
    >
      <SimpleGrid width="100%" gap={4} columns={3}>
        <Center width="100%">
          <ChakraLink to="/wallet">
            <IconButton
              color={(pathname === '/wallet') ? 'blue.400' : secondaryIconColor[colorMode] }
              variant="unstyled"
              size="md"
              aria-label="Wallet"
              fontSize="xl"
              icon={<RiCopperCoinFill />}
              display="flex"
            />
          </ChakraLink>
        </Center>
        <Center width="100%">
          <ChakraLink to="/gallery">
            <IconButton
              color={(pathname === '/gallery') ? 'blue.400' : secondaryIconColor[colorMode] }
              variant="unstyled"
              size="md"
              aria-label="Gallery"
              icon={<IoMdImage />}
              fontSize="xl"
              display="flex"
            />
          </ChakraLink>
        </Center>
        <Center width="100%">
          <ChakraLink to="/account">
            <IconButton
              color={(pathname === '/account') ? 'blue.400' : secondaryIconColor[colorMode] }
              variant="unstyled"
              size="md"
              aria-label="Account"
              icon={<IoIosPerson />}
              fontSize="xl"
              display="flex"
            />
          </ChakraLink>
        </Center>
      </SimpleGrid>
    </Center>
  )
}
