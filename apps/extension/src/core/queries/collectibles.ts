// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosClient,
  TokenClient,
  Types,
} from 'aptos';
import axios from 'axios';
import { useQuery, UseQueryOptions } from 'react-query';
import { validStorageUris } from 'core/constants';
import { MetadataJson } from 'core/types/tokenMetadata';
import { useCallback } from 'react';
import { getTokenIdDictFromString, getTokenIdStringFromDict, TokenId } from 'core/utils/token';
import {
  getEntryFunctionTransactions,
  getTransactionEvents,
} from 'core/queries/transaction';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { useNetworks } from 'core/hooks/useNetworks';

export const collectiblesQueryKeys = Object.freeze({
  getDepositTokens: 'getDepositTokens',
  getGalleryItems: 'getGalleryItems',
  getTokenData: 'getTokenData',
  isValidMetadataStructure: 'isValidMetadataStructure',
} as const);

export interface TokenAttributes {
  description?: string;
  id?: TokenId,
  imageUri?: string;
  metadata?: MetadataJson,
  name: string;
  supply?: number;
  uri: string;
}

type CollectionDict = Record<string, TokenAttributes[]>;

export function useDepositTokens(
  options?: UseQueryOptions<TokenAttributes[]>,
) {
  const { activeAccountAddress } = useActiveAccount();
  const { aptosClient } = useNetworks();

  async function getDepositTokens() {
    const events = await getTransactionEvents(
      aptosClient,
      activeAccountAddress,
      ['0x3::token::DepositEvent', '0x3::token::WithdrawEvent'],
    );
    const collectionDict: CollectionDict = {};
    const tokenClient = new TokenClient(aptosClient);
    const tokenCounterDict: Record<string, number> = {};
    const eventsToBeLoaded: TokenId[] = [];

    // Determine if the events have more deposits than withdrawals
    events.forEach((event) => {
      const { collection, creator, name } = event.data.id.token_data_id;
      const tokenString = getTokenIdStringFromDict({ collection, creator, name });
      if (!(tokenString in tokenCounterDict)) {
        tokenCounterDict[tokenString] = 0;
      }

      if (event.type === '0x3::token::DepositEvent') {
        tokenCounterDict[tokenString] += 1;
      } else if (event.type === '0x3::token::WithdrawEvent') {
        tokenCounterDict[tokenString] -= 1;
      }
    });
    Object.keys(tokenCounterDict).forEach((key) => {
      if (tokenCounterDict[key] > 0) {
        eventsToBeLoaded.push(getTokenIdDictFromString({ tokenId: key }));
      }
    });

    // Load metadata and image data for each event
    await Promise.all(eventsToBeLoaded.map(async (tokenId) => {
      const { collection, creator, name } = tokenId;
      if (!(collection in collectionDict)) {
        collectionDict[collection] = [];
      }

      let result;
      let uri = '';
      let metadata;
      try {
        result = await tokenClient.getTokenData(creator, collection, name);
        uri = result.uri;
        metadata = (await axios.get<MetadataJson>(uri)).data;
      } catch (error) {
        // eslint-disable-next-line no-console
        console.warn(error);
      }

      collectionDict[collection].push({
        id: {
          collection,
          creator,
          name,
        },
        imageUri: uri,
        metadata: {
          animation_url: metadata?.animation_url,
          attributes: metadata?.attributes,
          collection: metadata?.collection,
          description: metadata?.description ?? result?.description ?? '',
          external_url: metadata?.external_url,
          image: metadata?.image ?? uri,
          name: metadata?.name ?? name,
          properties: metadata?.properties,
          seller_fee_basis_points: metadata?.seller_fee_basis_points,
          symbol: metadata?.symbol,
        },
        name,
        uri,
      });
    }));

    return Array.from(Object.values(collectionDict)).flat(1).sort((a, b) => {
      if (a.name && b.name) {
        return a.name.localeCompare(b.name);
      }
      return 0;
    });
  }

  return useQuery<TokenAttributes[]>(
    [collectiblesQueryKeys.getDepositTokens],
    getDepositTokens,
    {
      ...options,
      enabled: Boolean(activeAccountAddress && aptosClient) && options?.enabled,
    },
  );
}

export function useGalleryItems(
  options?: UseQueryOptions<TokenAttributes[]>,
) {
  const { activeAccountAddress } = useActiveAccount();
  const { aptosClient } = useNetworks();

  async function getGalleryItems() {
    const createTokenTxns = await getEntryFunctionTransactions(
      aptosClient!,
      activeAccountAddress!,
      '0x3::token::create_token_script',
    );

    const collectionDict: CollectionDict = {};

    await Promise.all(createTokenTxns.map(async (txn) => {
      const payload = txn.payload as Types.EntryFunctionPayload;

      // TODO: do we need to go through HexString to deserialize the parameters?
      const creator = txn.sender;
      const collectionName = payload.arguments[0];
      const name = payload.arguments[1];
      const uri = payload.arguments[5];

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
  }

  return useQuery<TokenAttributes[]>(
    [collectiblesQueryKeys.getGalleryItems],
    getGalleryItems,
    {
      ...options,
      enabled: Boolean(activeAccountAddress && aptosClient) && options?.enabled,
    },
  );
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

async function getTokenData(aptosClient: AptosClient, tokenId: string) {
  const tokenClient = new TokenClient(aptosClient);
  const { collection, creator, name } = getTokenIdDictFromString({ tokenId });
  const tokenData = await tokenClient.getTokenData(creator, collection, name);

  // Cast as AxiosResponse of type TokenDataResponse
  const reformattedTokenData = (
    tokenData as unknown as TokenDataResponse
  );
  reformattedTokenData.collection = reformattedTokenData.collection || collection;

  // Get Arweave / IPFS link
  const tokenMetadata = await axios.get<MetadataJson>(reformattedTokenData.uri);
  reformattedTokenData.metadata = tokenMetadata.data;
  return reformattedTokenData;
}

export const useTokenData = (tokenId: string | undefined) => {
  const { aptosClient } = useNetworks();

  return useQuery(
    [collectiblesQueryKeys.getTokenData, tokenId],
    async () => getTokenData(aptosClient!, tokenId!),
    { enabled: Boolean(aptosClient && tokenId) },
  );
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
