// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react'
import { AptosAccount } from 'aptos'
import { Buffer } from 'buffer'
import {
  useNavigate
} from 'react-router-dom'
import useWalletState from '../hooks/useWalletState'
import {
  Box,
  Button,
  Center,
  Flex,
  Heading,
  Input,
  InputGroup,
  InputRightAddon,
  Text,
  useColorMode,
  VStack
} from '@chakra-ui/react'
import { SubmitHandler, useForm } from 'react-hook-form'
import { AptosBlackLogo, AptosWhiteLogo } from '../components/AptosLogo'
import withSimulatedExtensionContainer from '../components/WithSimulatedExtensionContainer'
import { QuestionIcon } from '@chakra-ui/icons'
import { getAccountResources } from './Wallet'

type Inputs = Record<string, any>

export const secondaryBgColor = {
  dark: 'gray.900',
  light: 'white'
}

const secondaryErrorMessageColor = {
  dark: 'red.200',
  light: 'red.500'
}

export const secondaryTextColor = {
  dark: 'gray.400',
  light: 'gray.500'
}

function Login () {
  const { colorMode } = useColorMode()
  const { updateWalletState } = useWalletState()
  const { register, watch, handleSubmit, setError, formState: { errors } } = useForm()
  const key: string = watch('privateKey')
  const navigate = useNavigate()

  const onSubmit: SubmitHandler<Inputs> = async (data, event) => {
    event?.preventDefault()
    try {
      const encodedKey = Uint8Array.from(Buffer.from(key, 'hex'))
      const account = new AptosAccount(encodedKey, undefined)
      const response = await getAccountResources({ address: account.address().hex() })
      if (response?.status !== 200) {
        setError('privateKey', { type: 'custom', message: 'Account not found' })
        return
      }
      updateWalletState({ aptosAccountState: account })
      navigate('/wallet')
    } catch (err) {
      setError('privateKey', { type: 'custom', message: 'Invalid private key' })
    }
  }

  return (
    <VStack
      bgColor={secondaryBgColor[colorMode]}
      justifyContent="center"
      spacing={4}
      width="100%"
      height="100%"
    >
      <Flex w="100%" flexDir="column">
        <Center>
          <Box width="75px" pb={4}>
            {
              (colorMode === 'dark')
                ? <AptosWhiteLogo />
                : <AptosBlackLogo />
            }
          </Box>
        </Center>
        <Heading textAlign="center">Wallet</Heading>
        <Text textAlign="center" pb={8} color={secondaryTextColor[colorMode]}>The official Aptos crypto wallet</Text>
        <form onSubmit={handleSubmit(onSubmit)}>
          <VStack spacing={4}>
            <Center minW="100%" px={4}>
              <Box>
                <InputGroup>
                  <Input
                    maxW="350px"
                    { ...register('privateKey')}
                    variant="filled"
                    required
                    placeholder='Private key...'
                  />
                  <InputRightAddon>
                    <Button type='submit' variant="unstyled">
                      Submit
                    </Button>
                  </InputRightAddon>
                </InputGroup>
                <Center>
                  <Text fontSize="xs" color={secondaryErrorMessageColor[colorMode]}>
                    {(errors?.privateKey?.message)}
                  </Text>
                </Center>
              </Box>
            </Center>
            <Button colorScheme="teal" variant="ghost" isDisabled>
              Create a new wallet
            </Button>
          </VStack>
        </form>
      </Flex>
      <Button size="xs" as="a" href='/help' leftIcon={<QuestionIcon />} variant="link">
        Help
      </Button>
    </VStack>
  )
}

export default withSimulatedExtensionContainer(Login)
