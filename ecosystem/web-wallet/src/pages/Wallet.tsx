// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
  Heading,
  HStack,
  Input,
  InputGroup,
  InputRightAddon,
  Popover,
  PopoverArrow,
  PopoverBody,
  PopoverContent,
  PopoverTrigger as OrigPopoverTrigger,
  Text,
  useColorMode,
  useDisclosure,
  VStack
} from '@chakra-ui/react'
import { SubmitHandler, useForm } from 'react-hook-form'
import React, { useEffect, useState } from 'react'
import WalletHeader, { seconaryAddressFontColor } from '../components/WalletHeader'
import withSimulatedExtensionContainer from '../components/WithSimulatedExtensionContainer'
import useWalletState from '../hooks/useWalletState'
import { AptosAccount, AptosClient, FaucetClient, Types } from 'aptos'
import { AccountResource } from 'aptos/dist/api/data-contracts'
import { FaFaucet } from 'react-icons/fa'
import { IoIosSend } from 'react-icons/io'
import numeral from 'numeral'
import { secondaryBgColor } from './Login'
import { devnetNodeUrl, faucetUrl } from '../constants'

/**
 * TODO: Will be fixed in upcoming Chakra-UI 2.0.0
 * @see https://github.com/chakra-ui/chakra-ui/issues/5896
 */
export const PopoverTrigger: React.FC<{ children: React.ReactNode }> =
  OrigPopoverTrigger

interface GetAccountResourcesProps {
  nodeUrl?: string;
  address?: string;
}

export const getAccountResources = async ({
  nodeUrl = devnetNodeUrl,
  address
}: GetAccountResourcesProps) => {
  const client = new AptosClient(nodeUrl)
  if (address) {
    const accountResources = await client.accounts.getAccountResources(
      address
    )
    return accountResources
  }
  return undefined
}

type Inputs = Record<string, any>

interface FundWithFaucetProps {
  nodeUrl?: string;
  address?: string;
}

const fundWithFaucet = async ({
  nodeUrl = devnetNodeUrl,
  address
}: FundWithFaucetProps): Promise<void> => {
  const faucetClient = new FaucetClient(nodeUrl, faucetUrl)
  if (address) {
    await faucetClient.fundAccount(address, 5000)
  }
}

interface SubmitTransactionProps {
  toAddress: string;
  fromAddress: AptosAccount;
  amount: string;
  nodeUrl?: string;
}

const submitTransaction = async ({
  toAddress,
  fromAddress,
  amount,
  nodeUrl = devnetNodeUrl
}: SubmitTransactionProps) => {
  const client = new AptosClient(nodeUrl)
  const payload: Types.TransactionPayload = {
    type: 'script_function_payload',
    function: '0x1::TestCoin::transfer',
    type_arguments: [],
    arguments: [toAddress, `${amount}`]
  }
  const txnRequest = await client.generateTransaction(fromAddress.address(), payload)
  const signedTxn = await client.signTransaction(fromAddress, txnRequest)
  const transactionRes = await client.submitTransaction(fromAddress, signedTxn)
  await client.waitForTransaction(transactionRes.hash)
}

const Wallet = () => {
  const { colorMode } = useColorMode()
  const { aptosAccount } = useWalletState()
  const { register, watch, handleSubmit } = useForm()
  const { onOpen, onClose, isOpen } = useDisclosure()
  const [accountResources, setAccountResources] = useState<AccountResource[] | undefined>(undefined)
  const [refreshState, setRefreshState] = useState(true)
  const [isFaucetLoading, setIsFaucetLoading] = useState(false)
  const [isTransferLoading, setIsTransferLoading] = useState(false)

  const address = aptosAccount?.address().hex()
  const accountResource = (accountResources) ? accountResources?.find((r) => r.type === '0x1::TestCoin::Balance') : undefined
  const tokenBalance = (accountResource) ? (accountResource.data as { coin: { value: string } }).coin.value : undefined
  const tokenBalanceString = numeral(tokenBalance).format('0,0.0000')
  const toAddress: string | undefined | null = watch('toAddress')
  const transferAmount: string | undefined | null = watch('transferAmount')

  const onSubmit: SubmitHandler<Inputs> = async (data, event) => {
    event?.preventDefault()
    if (toAddress && aptosAccount && transferAmount) {
      setIsTransferLoading(true)
      await submitTransaction({
        toAddress,
        fromAddress: aptosAccount,
        amount: transferAmount
      })
      setRefreshState(!refreshState)
      setIsTransferLoading(false)
      onClose()
    }
  }

  const faucetOnClick = async () => {
    setIsFaucetLoading(true)
    await fundWithFaucet({ address })
    setRefreshState(!refreshState)
    setIsFaucetLoading(false)
  }

  useEffect(() => {
    getAccountResources({ address })?.then((data) => {
      const tempAccountResources = data?.data
      setAccountResources(tempAccountResources)
    })
  }, [refreshState])

  return (
    <VStack
      height="100%"
      maxW="100%"
      width="100%"
      bgColor={secondaryBgColor[colorMode]}
    >
      <WalletHeader />
      <VStack width="100%" paddingTop={8}>
        <Text fontSize="sm" color={seconaryAddressFontColor[colorMode]}>Account balance</Text>
        <Heading>{tokenBalanceString}</Heading>
        <HStack spacing={4}>
          <Button
            isLoading={isFaucetLoading}
            leftIcon={<FaFaucet />}
            onClick={faucetOnClick}
            isDisabled={isFaucetLoading}
          >
            Faucet
          </Button>
          <Popover
            isOpen={isOpen}
            onOpen={onOpen}
            onClose={onClose}
          >
            <PopoverTrigger>
              <Button
                isLoading={isTransferLoading}
                isDisabled={isTransferLoading}
                leftIcon={<IoIosSend />}
              >
                Send
              </Button>
            </PopoverTrigger>
            <PopoverContent>
              <PopoverArrow />
              <PopoverBody>
                <form onSubmit={handleSubmit(onSubmit)}>
                  <VStack spacing={4}>
                    <InputGroup>
                      <Input
                        variant="filled"
                        placeholder="To address"
                        {...register('toAddress')}
                      />
                    </InputGroup>
                    <InputGroup>
                      <Input
                        type="number"
                        variant="filled"
                        placeholder="Transfer amount"
                        {...register('transferAmount')}
                      />
                      <InputRightAddon>
                        tokens
                      </InputRightAddon>
                    </InputGroup>
                    <Button isDisabled={isTransferLoading} type="submit">
                      Submit
                    </Button>
                  </VStack>
                </form>
              </PopoverBody>
            </PopoverContent>
          </Popover>
        </HStack>
      </VStack>
    </VStack>
  )
}

export default withSimulatedExtensionContainer(Wallet)
