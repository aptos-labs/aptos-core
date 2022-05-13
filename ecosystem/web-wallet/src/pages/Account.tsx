// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Heading,
  useColorMode,
  VStack,
  Button,
  Flex,
  SimpleGrid,
  Tooltip,
  useClipboard,
  Text,
  Tag,
  Modal,
  useDisclosure,
  ModalOverlay,
  ModalContent,
  ModalHeader,
  ModalCloseButton,
  ModalBody
} from '@chakra-ui/react'
import React from 'react'
import withSimulatedExtensionContainer from '../components/WithSimulatedExtensionContainer'
import { CredentialHeaderAndBody, CredentialHeaderAndBodyProps } from './CreateWallet'
import useWalletState from '../hooks/useWalletState'
import { useNavigate } from 'react-router-dom'
import { secondaryTextColor } from './Login'
import { ExternalLinkIcon } from '@chakra-ui/icons'
import WalletLayout from '../Layouts/WalletLayout'

export const CredentialRow = ({
  header,
  body
}: CredentialHeaderAndBodyProps) => {
  const { hasCopied, onCopy } = useClipboard(body || '')
  const { colorMode } = useColorMode()
  return (
    <SimpleGrid columns={2} width="100%">
      <Flex alignItems="flex-start">
        <Text fontSize="xs" color={secondaryTextColor[colorMode]}>
          {header}
        </Text>
      </Flex>
      <Flex alignItems="flex-end">
        <Tooltip label={hasCopied ? 'Copied!' : 'Copy'} closeDelay={300}>
          <Text fontSize="xs" cursor="pointer" noOfLines={1} onClick={onCopy}>
            {body}
          </Text>
        </Tooltip>
      </Flex>
    </SimpleGrid>
  )
}

const Account = () => {
  const { isOpen, onOpen, onClose } = useDisclosure()
  const { signOut, aptosAccount } = useWalletState()
  const navigate = useNavigate()

  const privateKeyObject = aptosAccount?.toPrivateKeyObject()
  const privateKeyHex = privateKeyObject?.privateKeyHex
  const publicKeyHex = privateKeyObject?.publicKeyHex
  const address = privateKeyObject?.address
  const explorerAddress = `https://explorer.devnet.aptos.dev/account/${address}`

  const signOutOnClick = () => {
    signOut()
    navigate('/')
  }

  return (
    <WalletLayout>
      <VStack width="100%" paddingTop={8}>
        <Box px={4} pb={4}>
          <Heading fontSize="xl">Account</Heading>
          <Flex pb={2} pt={1}>
            <Button
              fontSize="sm"
              fontWeight={400}
              as="a"
              target="_blank"
              rightIcon={<ExternalLinkIcon />}
              variant="unstyled"
              cursor="pointer"
              href={explorerAddress}
            >
              View on explorer
            </Button>
          </Flex>
          <SimpleGrid columns={2} width="100%">
            <Flex>
              <Heading fontSize="sm">Credentials</Heading>
            </Flex>
            <Flex justifyContent="right">
              <Tag size="sm" onClick={onOpen} cursor="pointer">
                View more
              </Tag>
              <Modal isOpen={isOpen} onClose={onClose}>
                <ModalOverlay />
                <ModalContent>
                  <ModalHeader>
                    Account Credentials
                  </ModalHeader>
                  <ModalCloseButton />
                  <ModalBody>
                  <VStack mt={2} spacing={4} pb={8}>
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
                  </ModalBody>
                </ModalContent>
              </Modal>
            </Flex>
          </SimpleGrid>
          <VStack mt={2} spacing={2} alignItems="left">
            <CredentialRow
              header="Private key"
              body={privateKeyHex}
            />
            <CredentialRow
              header="Public key"
              body={publicKeyHex}
            />
            <CredentialRow
              header="Address"
              body={address}
            />
          </VStack>
          <Box pt={4}>
            <Button onClick={signOutOnClick} colorScheme="red" size="sm">
              Sign out
            </Button>
          </Box>
        </Box>
      </VStack>
    </WalletLayout>
  )
}

export default withSimulatedExtensionContainer(Account)
