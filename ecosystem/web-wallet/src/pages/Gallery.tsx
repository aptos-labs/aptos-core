// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Center,
  Flex,
  Grid,
  Heading,
  SimpleGrid,
  Text,
  useColorMode,
  VStack
} from '@chakra-ui/react'
import { AptosClient, HexString } from 'aptos'
import React from 'react'
import CreateNFTModal from '../components/CreateNFTModal'
import GalleryItem from '../components/GalleryItem'
import withSimulatedExtensionContainer from '../components/WithSimulatedExtensionContainer'
import { NODE_URL, validStorageUris } from '../constants'
import useWalletState from '../hooks/useWalletState'
import WalletLayout from '../Layouts/WalletLayout'
import axios from 'axios'
import { MetadataJson } from '../types/TokenMetadata'
import { useQuery } from 'react-query'
import { AptosAccountState } from '../types'
import SquareBox from '../components/SquareBox'

interface TokenAttributes {
  name: string;
  description?: string;
  uri: string;
  imageUri?: string;
  supply?: number;
  metadata?: MetadataJson
}

type CollectionDict = Record<string, TokenAttributes[]>
type StorageDict = Record<string, MetadataJson>

interface GetGalleryItemsProps {
  aptosAccount: AptosAccountState
}

const secondaryBorderColor = {
  light: 'gray.200',
  dark: 'gray.600'
}

// this is a temporary workaround until we get the indexer working
const getGalleryItems = async ({
  aptosAccount
}: GetGalleryItemsProps) => {
  if (!aptosAccount) {
    return undefined
  }
  const aptosClient = new AptosClient(NODE_URL)
  const hexAddress = aptosAccount?.address().hex()
  if (hexAddress) {
    const collectionDict: CollectionDict = {}
    const storageDict: StorageDict = {}
    const accountTransactions = (await aptosClient.getAccountTransactions(hexAddress)).filter((txn) => (
      !txn?.vm_status?.includes('Move abort')
    ))
    accountTransactions.forEach((transaction) => {
      if ('payload' in transaction && 'function' in transaction.payload) {
        if (transaction?.payload?.function === '0x1::Token::create_unlimited_collection_script') {
          const collectionName = new HexString(transaction.payload.arguments[0]).toBuffer().toString()
          collectionDict[collectionName] = []
        }
      }
    })

    const storageUris = await Promise.all(accountTransactions.map(async (accountTransaction) => {
      if (
        'payload' in accountTransaction &&
        'function' in accountTransaction.payload &&
        accountTransaction.payload.function === '0x1::Token::create_unlimited_token_script'
      ) {
        const uri = new HexString(accountTransaction.payload.arguments[5]).toBuffer().toString()
        // check if uri is hosted on ipfs, arweave, or s3
        if (validStorageUris.some(v => uri.includes(v))) {
          // Will need to re-examine this type in the future
          const fetchedUrl = axios.get<MetadataJson>(uri)
          return fetchedUrl
        }
      }
      return undefined
    }))

    storageUris.forEach((value) => {
      if (value !== undefined && value.config.url?.toString()) {
        storageDict[value.config.url.toString()] = value.data
      }
    })

    for (const accountTransaction of accountTransactions) {
      if (
        'payload' in accountTransaction &&
        'function' in accountTransaction.payload &&
        accountTransaction.payload.function === '0x1::Token::create_unlimited_token_script'
      ) {
        const collectionName = new HexString(accountTransaction.payload.arguments[0]).toBuffer().toString()
        const name = new HexString(accountTransaction.payload.arguments[1]).toBuffer().toString()
        const uri = new HexString(accountTransaction.payload.arguments[5]).toBuffer().toString()
        collectionDict[collectionName].push({
          name,
          uri,
          metadata: storageDict[uri]
        })
      }
    }
    const flatMap = Array.from(Object.values(collectionDict)).flat(1)
    return flatMap
  }
}

const Gallery = () => {
  const { aptosAccount } = useWalletState()
  const { colorMode } = useColorMode()
  const {
    data: galleryItems
  } = useQuery('gallery-items', () => getGalleryItems({ aptosAccount }))

  return (
    <WalletLayout>
      <VStack width="100%" paddingTop={8} px={4}>
        <Grid pb={4} templateColumns="1fr 72px" width="100%">
          <Heading fontSize="xl" >Collectibles</Heading>
          <Flex justifyContent="right">
            <CreateNFTModal />
          </Flex>
        </Grid>
        <SimpleGrid w="100%" columns={2} spacing={2}>
          {
            (galleryItems && galleryItems.length > 0)
              ? (
                  galleryItems?.map((item, index) => (
                    <GalleryItem
                      key={`${item.name}-${index}`}
                      title={item.name}
                      imageSrc={item.metadata?.image || 'https://www.publicdomainpictures.net/pictures/280000/nahled/not-found-image-15383864787lu.jpg'}
                    />
                  ))
                )
              : (
              <SquareBox borderWidth="1px" borderRadius=".5rem" borderColor={secondaryBorderColor[colorMode]}>
                <Center height="100%" p={4}>
                  <Text textAlign="center">No collectibles yet!</Text>
                </Center>
              </SquareBox>
                )
          }
        </SimpleGrid>
      </VStack>
    </WalletLayout>
  )
}

export default withSimulatedExtensionContainer(Gallery)
