// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, HexString } from 'aptos';
import axios from 'axios';
import { useQuery } from 'react-query';
import { validStorageUris } from 'core/constants';
import useWalletState from 'core/hooks/useWalletState';
import { MetadataJson } from 'core/types/TokenMetadata';
import { AptosAccountState } from 'core/types';
import { AptosNetwork } from 'core/utils/network';
import { useCallback } from 'react';

interface TokenAttributes {
  description?: string;
  imageUri?: string;
  metadata?: MetadataJson,
  name: string;
  supply?: number;
  uri: string;
}

type CollectionDict = Record<string, TokenAttributes[]>;
type StorageDict = Record<string, MetadataJson>;

interface GetGalleryItemsProps {
  aptosAccount: AptosAccountState;
  nodeUrl: AptosNetwork;
}

// this is a temporary workaround until we get the indexer working
export const getGalleryItems = async ({
  aptosAccount,
  nodeUrl,
}: GetGalleryItemsProps) => {
  if (!aptosAccount) {
    return undefined;
  }
  const aptosClient = new AptosClient(nodeUrl);
  const hexAddress = aptosAccount?.address().hex();
  if (hexAddress) {
    const collectionDict: CollectionDict = {};
    const storageDict: StorageDict = {};
    const accountTransactions = (await aptosClient
      .getAccountTransactions(hexAddress)).filter((txn) => (
      !txn?.vm_status?.includes('Move abort')
    ));
    accountTransactions.forEach((transaction) => {
      if ('payload' in transaction && 'function' in transaction.payload) {
        if (transaction?.payload?.function === '0x1::Token::create_unlimited_collection_script') {
          const collectionName = new HexString(
            transaction.payload.arguments[0],
          ).toBuffer().toString();
          collectionDict[collectionName] = [];
        }
      }
    });

    const storageUris = await Promise.all(accountTransactions.map(async (accountTransaction) => {
      if (
        'payload' in accountTransaction
        && 'function' in accountTransaction.payload
        && accountTransaction.payload.function === '0x1::Token::create_unlimited_token_script'
      ) {
        const uri = new HexString(accountTransaction.payload.arguments[5]).toBuffer().toString();
        // check if uri is hosted on ipfs, arweave, or s3
        if (validStorageUris.some((v) => uri.includes(v))) {
          // Will need to re-examine this type in the future
          const fetchedUrl = axios.get<MetadataJson>(uri);
          return fetchedUrl;
        }
      }
      return undefined;
    }));

    storageUris.forEach((value) => {
      if (value !== undefined && value.config.url?.toString()) {
        storageDict[value.config.url.toString()] = value.data;
      }
    });

    accountTransactions.forEach((accountTransaction) => {
      if (
        'payload' in accountTransaction
        && 'function' in accountTransaction.payload
        && accountTransaction.payload.function === '0x1::Token::create_unlimited_token_script'
      ) {
        const collectionName = new HexString(
          accountTransaction.payload.arguments[0],
        ).toBuffer().toString();
        const name = new HexString(
          accountTransaction.payload.arguments[1],
        ).toBuffer().toString();
        const uri = new HexString(
          accountTransaction.payload.arguments[5],
        ).toBuffer().toString();
        collectionDict[collectionName].push({
          metadata: storageDict[uri],
          name,
          uri,
        });
      }
    });
    const flatMap = Array.from(Object.values(collectionDict)).flat(1);
    return flatMap;
  }
  return undefined;
};

export const collectiblesQueryKeys = Object.freeze({
  getGalleryItems: 'getGalleryItems',
} as const);

export const useGalleryItems = () => {
  const {
    aptosAccount, aptosNetwork,
  } = useWalletState();

  const getGalleryItemsQuery = useCallback(async () => {
    const galleryItems = await getGalleryItems({ aptosAccount, nodeUrl: aptosNetwork });
    return galleryItems;
  }, [aptosAccount, aptosNetwork]);

  return useQuery(collectiblesQueryKeys.getGalleryItems, getGalleryItemsQuery);
};
