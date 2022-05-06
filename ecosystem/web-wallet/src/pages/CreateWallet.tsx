// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Button,
  Flex,
  Heading,
  Tag,
  Text,
  Tooltip,
  useClipboard,
  useColorMode,
  VStack
} from '@chakra-ui/react'
import { FaucetClient } from 'aptos'
import React, { useState } from 'react'
import ChakraLink from '../components/ChakraLink'
import CreateWalletHeader from '../components/CreateWalletHeader'
import withSimulatedExtensionContainer from '../components/WithSimulatedExtensionContainer'
import { NODE_URL, FAUCET_URL, secondaryBgColor } from '../constants'
import useWalletState from '../hooks/useWalletState'
import { createNewAccount } from '../utils/account'

export interface CredentialHeaderAndBodyProps {
  header: string;
  body: string | undefined;
}

export const CredentialHeaderAndBody = ({
  header,
  body
}: CredentialHeaderAndBodyProps) => {
  const { hasCopied, onCopy } = useClipboard(body || '')
  return (
    <VStack spacing={2} maxW="100%" alignItems="flex-start">
      <Tag>
        {header}
      </Tag>
      <Tooltip label={hasCopied ? 'Copied!' : 'Copy'} closeDelay={300}>
        <Text fontSize="xs" cursor="pointer" wordBreak="break-word" onClick={onCopy}>
          {body}
        </Text>
      </Tooltip>
    </VStack>
  )
}

const NewAccountState = () => {
  const [isAccountBeingCreated, setIsAccountBeingCreated] = useState<boolean>(false)
  const { updateWalletState, aptosAccount } = useWalletState()
  const privateKeyObject = aptosAccount?.toPrivateKeyObject()
  const privateKeyHex = privateKeyObject?.privateKeyHex
  const publicKeyHex = privateKeyObject?.publicKeyHex
  const address = privateKeyObject?.address

  const createAccountOnClick = async () => {
    setIsAccountBeingCreated(true)
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL)
    const account = createNewAccount()
    await faucetClient.fundAccount(account.address(), 0)
    updateWalletState({ aptosAccountState: account })
    setIsAccountBeingCreated(false)
  }

  return (
    <Box px={4} pb={4}>
      <Box maxW="100%">
        {
          (!aptosAccount)
            ? (
                <>
                  <Heading fontSize="xl" pb={4}>New account</Heading>
                  <Text fontSize="sm" maxW="100%" wordBreak="break-word">
                    If you do not have a wallet account, you can create a private / public key account by clicking the button below
                  </Text>
                  <Box pt={4}>
                    <Button isLoading={isAccountBeingCreated} size="sm" colorScheme="teal" onClick={createAccountOnClick}>
                      Create Account
                    </Button>
                  </Box>
                </>
              )
            : (
                <>
                  <Heading fontSize="xl" pb={4}>Account credentials</Heading>
                  <Text fontSize="sm" maxW="100%" wordBreak="break-word">
                    Please DO NOT lose these credentials, this is the only time you will be able to store your them.
                  </Text>
                  <VStack mt={4} spacing={4}>
                    <CredentialHeaderAndBody
                      header="Private key"
                      body={privateKeyHex}
                    />
                    <CredentialHeaderAndBody
                      header="Public key"
                      body={publicKeyHex}
                    />
                    <CredentialHeaderAndBody
                      header="Address"
                      body={address}
                    />
                  </VStack>
                  <Flex width="100%" pt={4}>
                    <ChakraLink to="/">
                      <Button colorScheme="teal" size="sm">
                        Proceed to wallet
                      </Button>
                    </ChakraLink>
                  </Flex>
                </>
              )
        }
      </Box>
    </Box>
  )
}

const CreateWallet = () => {
  const { colorMode } = useColorMode()

  return (
    <VStack
      bgColor={secondaryBgColor[colorMode]}
      spacing={4}
      width="100%"
      height="100%"
    >
      <CreateWalletHeader />
      <VStack width="100%" pt={4}>
        <NewAccountState />
      </VStack>
    </VStack>
  )
}

export default withSimulatedExtensionContainer(CreateWallet)
