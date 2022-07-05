// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosClient, HexString,
  MaybeHexString, TokenClient,
} from 'aptos';
import axios from 'axios';
import { useQuery } from 'react-query';
import { validStorageUris } from 'core/constants';
import useWalletState from 'core/hooks/useWalletState';
import { MetadataJson } from 'core/types/TokenMetadata';
import { AptosNetwork } from 'core/utils/network';
import { useCallback, useMemo } from 'react';
import { getTokenIdDictFromString, TokenId } from 'core/utils/token';
import { ScriptFunctionPayload } from 'aptos/dist/api/data-contracts';
import { getScriptFunctionTransactions } from 'core/queries/transaction';

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

interface GetGalleryItemsProps {
  address: MaybeHexString;
  nodeUrl: AptosNetwork;
}

// this is a temporary workaround until we get the indexer working
export const getGalleryItems = async ({
  address,
  nodeUrl,
}: GetGalleryItemsProps) => {
  const createTokenTransactions = await getScriptFunctionTransactions({
    address,
    functionName: '0x1::Token::create_unlimited_token_script',
    nodeUrl,
  });

  const collectionDict: CollectionDict = {};

  // Note: next block is currently not needed. Maybe it could be
  // helpful in the future to support showing empty collections?

  // Initialize collection dict from `create_unlimited_collection` transactions
  // const createCollectionTransactions = await getScriptFunctionTransactions({
  //   address,
  //   functionName: '0x1::Token::create_unlimited_collection_script',
  //   nodeUrl,
  // });
  //
  // createCollectionTransactions.forEach((txn) => {
  //   const payload = txn.payload as ScriptFunctionPayload;
  //   const collectionName = new HexString(payload.arguments[0]).toBuffer().toString();
  //   collectionDict[collectionName] = [];
  // });

  await Promise.all(createTokenTransactions.map(async (txn) => {
    const payload = txn.payload as ScriptFunctionPayload;

    // TODO: do we need to go through HexString to deserialize the parameters?
    const creator = txn.sender;
    const collectionName = new HexString(payload.arguments[0]).toBuffer().toString();
    const name = new HexString(payload.arguments[1]).toBuffer().toString();
    const uri = new HexString(payload.arguments[5]).toBuffer().toString();

    const isSupportedStorage = validStorageUris.some((storageUri) => uri.includes(storageUri));
    const metadata = isSupportedStorage ? (await axios.get<MetadataJson>(uri)).data : undefined;

    if (!(collectionName in collectionDict)) {
      collectionDict[collectionName] = [];
    }

    collectionDict[collectionName].push({
      id: {
        collection: collectionName,
        creator,
        name,
      },
      metadata,
      name,
      uri,
    });
  }));

  return Array.from(Object.values(collectionDict)).flat(1);
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

  const getGalleryItemsQuery = useCallback(
    async () => (aptosAccount ? getGalleryItems({
      address: aptosAccount.address(),
      nodeUrl: aptosNetwork,
    }) : null),
    [aptosAccount, aptosNetwork],
  );

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
