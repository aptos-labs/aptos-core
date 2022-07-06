// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, HexString, TokenClient } from 'aptos';
import axios from 'axios';
import { useQuery } from 'react-query';
import { validStorageUris } from 'core/constants';
import useWalletState from 'core/hooks/useWalletState';
import { MetadataJson } from 'core/types/TokenMetadata';
import { AptosAccountState } from 'core/types';
import { AptosNetwork } from 'core/utils/network';
import { useCallback, useMemo } from 'react';
import { getTokenIdDictFromString, TokenId } from 'core/utils/token';

interface TokenAttributes {
  description?: string;
  id?: TokenId,
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
        const creator = (accountTransaction as any).sender;
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
          id: {
            collection: collectionName,
            creator,
            name,
          },
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
  getTokenData: 'getTokenData',
  isValidMetadataStructure: 'isValidMetadataStructure',
} as const);

interface GetTokenDataProps {
  id: TokenId;
  nodeUrl: string;
}

export const getTokenData = async ({
  id,
  nodeUrl,
}: GetTokenDataProps) => {
  const aptosClient = new AptosClient(nodeUrl);
  const tokenClient = new TokenClient(aptosClient);
  const data = await tokenClient.getTokenData(id.creator, id.collection, id.name);
  return data;
};

interface UseTokenDataProps {
  tokenId: string;
}

export interface TokenDataResponse {
  collection: string;
  description: string;
  maximum: {
    vec: number[]
  },
  metadata?: MetadataJson;
  name: string;
  supply: {
    vec: number[]
  };
  uri: string;
}

export const useTokenData = ({
  tokenId,
}: UseTokenDataProps) => {
  const { aptosNetwork } = useWalletState();
  const tokenIdDict = useMemo(() => getTokenIdDictFromString({ tokenId }), []);

  const getGalleryItemsQuery = useCallback(async () => {
    const tokenData = await getTokenData({
      id: tokenIdDict,
      nodeUrl: aptosNetwork,
    });

    // Cast as AxiosResponse of type TokenDataResponse
    const reformattedTokenData = (
      tokenData as unknown as TokenDataResponse
    );

    // Get Arweave / IPFS link
    const tokenMetadata = await axios.get<MetadataJson>(reformattedTokenData.uri);
    reformattedTokenData.metadata = tokenMetadata.data;
    return reformattedTokenData;
  }, [aptosNetwork, tokenIdDict]);

  return useQuery(collectiblesQueryKeys.getTokenData, getGalleryItemsQuery);
};

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

interface IsValidMetadataStructureProps {
  uri: string;
}

export const getIsValidMetadataStructure = async ({
  uri,
}: IsValidMetadataStructureProps) => {
  try {
    const { data } = await axios.get<MetadataJson>(uri);
    if (!(
      data.description
    && data.image
    && data.name
    && data.properties
    && data.seller_fee_basis_points
    && data.symbol
    )) {
      return false;
    }

    if (!(
      data.properties.category
    && data.properties.creators
    && data.properties.files
    )) {
      return false;
    }

    // eslint-disable-next-line no-restricted-syntax
    for (const creator of data.properties.creators) {
      if (!(creator.address && creator.share)) {
        return false;
      }
    }

    // eslint-disable-next-line no-restricted-syntax
    for (const file of data.properties.files) {
      if (!(
        file.type
      && file.uri
      )) {
        return false;
      }
    }

    return true;
  } catch (err) {
    return false;
  }
};

export const useIsValidMetadataStructure = ({
  uri,
}: IsValidMetadataStructureProps) => {
  const isValidMetadataStructureQuery = useCallback(async () => {
    const result = await getIsValidMetadataStructure({ uri });
    return result;
  }, [uri]);
  return useQuery(collectiblesQueryKeys.isValidMetadataStructure, isValidMetadataStructureQuery);
};
