/* eslint-disable no-unused-vars */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AddIcon } from '@chakra-ui/icons'
import { Button, Flex, Grid, Heading, SimpleGrid, VStack } from '@chakra-ui/react'
import { AptosClient, AptosAccount, type Types } from 'aptos'
import { Item } from 'framer-motion/types/components/Reorder/Item'
import React from 'react'
import GalleryItem from '../components/GalleryItem'
import withSimulatedExtensionContainer from '../components/WithSimulatedExtensionContainer'
import { NODE_URL } from '../constants'
import useWalletState from '../hooks/useWalletState'
import WalletLayout from '../Layouts/WalletLayout'

window.Buffer = window.Buffer || require('buffer').Buffer

// eslint-disable-next-line no-unused-vars
const createToken = async (
  account: AptosAccount,
  collectionName: string,
  name: string,
  description: string,
  supply: number,
  uri: string): Promise<Types.HexEncodedBytes> => {
  const payload: { function: string; arguments: any[]; type: string; type_arguments: any[] } = {
    type: 'script_function_payload',
    function: '0x1::Token::create_unlimited_token_script',
    type_arguments: [],
    arguments: [
      Buffer.from(collectionName).toString('hex'),
      Buffer.from(name).toString('hex'),
      Buffer.from(description).toString('hex'),
      true,
      supply.toString(),
      Buffer.from(uri).toString('hex')
    ]
  }
  const aptosClient = new AptosClient(NODE_URL)
  const txnRequest = await aptosClient.generateTransaction(account.address(), payload)
  const signedTxn = await aptosClient.signTransaction(account, txnRequest)
  const res = await aptosClient.submitTransaction(signedTxn)
  await aptosClient.waitForTransaction(res.hash)
  return Promise.resolve(res.hash)
}

const createCollection = async (
  account: AptosAccount,
  name: string,
  description: string,
  uri: string
): Promise<Types.HexEncodedBytes> => {
  const payload: Types.TransactionPayload = {
    type: 'script_function_payload',
    function: '0x1::Token::create_unlimited_collection_script',
    type_arguments: [],
    arguments: [
      Buffer.from(name).toString('hex'),
      Buffer.from(description).toString('hex'),
      Buffer.from(uri).toString('hex')
    ]
  }
  const aptosClient = new AptosClient(NODE_URL)
  const txnRequest = await aptosClient.generateTransaction(account.address(), payload)
  const signedTxn = await aptosClient.signTransaction(account, txnRequest)
  const res = await aptosClient.submitTransaction(signedTxn)
  await aptosClient.waitForTransaction(res.hash)
  return Promise.resolve(res.hash)
}

const tableItem = async (handle: string, keyType: string, valueType: string, key: any): Promise<any> => {
  const response = await fetch(`${NODE_URL}/tables/${handle}/item`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      key_type: keyType,
      value_type: valueType,
      key
    })
  })

  if (response.status === 404) {
    return null
  } else if (response.status !== 200) {
    // assert(response.status === 200, await response.text())
  } else {
    return await response.json()
  }
}

const getToken = async (
  aptosAccount: AptosAccount,
  aptosClient: AptosClient,
  tokenKey: string,
  collectionKey: string
) => {
  const accountResource = await aptosClient.getAccountResources(aptosAccount.address().hex())
  let collections: Types.AccountResource | undefined
  let tokens: Types.AccountResource | undefined
  for (const item of accountResource) {
    if (item.type === '0x1::Token::Collections') {
      collections = item
    } else if (item.type === '0x1::Token::TokenStore') {
      tokens = item
    }
  }
  if (tokens && collections) {
    const tokensHandle = (tokens.data as Record<string, any>).tokens.handle
    console.log(tokensHandle)
    const collectionsHandle = (collections.data as Record<string, any>).collections.handle
    console.log(collectionsHandle)

    const tokenResult = await tableItem(
      tokensHandle,
      '0x1::ASCII::String',
      '0x1::Token::Collection',
      tokenKey
    )
  }
  console.log(accountResource)
}

const Gallery = () => {
  const { aptosAccount } = useWalletState()
  const newOnClick = async () => {
    const aptosClient = new AptosClient(NODE_URL)
    // const tokenClient = new TokenClient(aptosClient)
    try {
      if (aptosAccount) {
        const collectionName = `${new Date().toDateString()}`
        // const result = getToken(
        //   aptosAccount,
        //   aptosClient,
        //   'USDC',
        //   ''
        // )
        const collectionTxnHash = await createCollection(
          aptosAccount,
          collectionName,
          'blank collection',
          'https://hariri.dev/'
        )
        console.log(collectionTxnHash)

        const tokenTxnHash = await createToken(
          aptosAccount,
          collectionName,
          'USDC',
          'A USD equivalent',
          100000,
          'https://hariri.dev'
        )

        console.log(tokenTxnHash)
        // TODO: temp fix: https://github.com/aptos-labs/aptos-core/pull/748/files#diff-a35123cdf0e1d8aa96b496b42612f8e0ed8f436b5a3cfc21244f5a3dc146f2f5R41
        // await createToken(
        //   aptosAccount,
        //   'New collection',
        //   'Token 1'
        // )
      }
    } catch (err) {
      console.log(err)
    }
  }
  return (
    <WalletLayout>
      <VStack width="100%" paddingTop={8} px={4}>
        <Grid pb={4} templateColumns="1fr 72px" width="100%">
          <Heading fontSize="xl" >Collectibles</Heading>
          <Flex justifyContent="right">
            <Button size="xs" onClick={newOnClick} leftIcon={<AddIcon fontSize="xs"/>}>
              New
            </Button>
          </Flex>
        </Grid>
        <SimpleGrid w="100%" columns={2} spacing={2}>
          {
            [1, 2, 3, 4, 5, 6].map((item) => (
              <GalleryItem
                key={`${item}`}
                title={`${item}`}
                imageSrc="https://d15shllkswkct0.cloudfront.net/wp-content/blogs.dir/1/files/2021/07/phantom.jpg"
              />
            ))
          }
        </SimpleGrid>
      </VStack>
    </WalletLayout>
  )
}

export default withSimulatedExtensionContainer(Gallery)
