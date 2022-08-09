// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosAccount,
  AptosClient,
  TokenClient,
  RequestError,
} from 'aptos';
import { getIsValidMetadataStructure } from 'core/queries/collectibles';
import queryKeys from 'core/queries/queryKeys';
import Analytics from 'core/utils/analytics/analytics';
import { collectiblesEvents, CombinedEventParams } from 'core/utils/analytics/events';
import { NodeUrl } from 'core/utils/network';
import { useCallback } from 'react';
import { useMutation, useQueryClient } from 'react-query';
import useGlobalStateContext from 'core/hooks/useGlobalState';

export const defaultRequestErrorAttributes = {
  config: {},
  headers: {},
  status: 400,
  statusText: 'Move abort',
};

const ERROR_CODES = Object.freeze({
  URI_GENERAL: 'URI is invalid',
  URI_METADATA_FORMAT: 'Wrong metadata format in URI',
} as const);

export interface RaiseForErrorProps {
  error?: string;
  vmStatus?: string
}

const raiseForError = ({
  error,
  vmStatus,
}: RaiseForErrorProps) => {
  if (error?.includes(ERROR_CODES.URI_METADATA_FORMAT)) {
    throw new RequestError(error, {
      data: {
        message: error,
      },
      ...defaultRequestErrorAttributes,
      statusText: error,
    });
  } else if (vmStatus?.includes('Move abort')) {
    throw new RequestError(vmStatus, {
      data: {
        message: vmStatus,
      },
      ...defaultRequestErrorAttributes,
    });
  }
};

interface CreateTokenAndCollectionProps {
  collectionName?: string;
  description?: string;
  name?: string;
  royalty_points_per_million?: number;
  supply: number;
  uri?: string;
}

export const createTokenAndCollection = async (
  account: AptosAccount,
  aptosClient: AptosClient,
  {
    collectionName,
    description,
    name,
    royalty_points_per_million = 0,
    supply,
    uri,
  }: CreateTokenAndCollectionProps,
) => {
  if (!account || !(collectionName && description && uri && name)) {
    return undefined;
  }
  const isValidUri = await getIsValidMetadataStructure({ uri });
  raiseForError({
    error: (isValidUri)
      ? undefined
      : `${ERROR_CODES.URI_METADATA_FORMAT} or ${ERROR_CODES.URI_GENERAL}`,
  });
  const tokenClient = new TokenClient(aptosClient);

  const collectionTxnHash = await tokenClient.createCollection(
    account,
    collectionName,
    description,
    uri,
  );

  // Move abort errors do not throw so we need to check them manually
  const collectionTxn: any = await aptosClient.getTransaction(collectionTxnHash);
  let vmStatus: string = collectionTxn.vm_status;
  raiseForError({ vmStatus });

  const tokenTxnHash = await tokenClient.createToken(
    account,
    collectionName,
    name,
    description,
    supply,
    uri,
    royalty_points_per_million,
  );
  const tokenTxn: any = await aptosClient.getTransaction(tokenTxnHash);
  vmStatus = tokenTxn.vm_status;
  raiseForError({ vmStatus });

  return {
    address: account.address().hex(),
    amount: 1,
    collection: collectionName,
    description,
    name,
    uri,
  };
};

export const useCreateTokenAndCollection = () => {
  const queryClient = useQueryClient();
  const {
    activeNetwork,
    aptosAccount,
    aptosClient,
  } = useGlobalStateContext();

  const createTokenAndCollectionOnSettled = useCallback(async (
    data: CombinedEventParams | undefined,
  ) => {
    queryClient.invalidateQueries(queryKeys.getGalleryItems);
    queryClient.invalidateQueries(queryKeys.getAccountCoinBalance);
    Analytics.event({
      eventType: collectiblesEvents.CREATE_NFT,
      params: {
        network: activeNetwork!.nodeUrl as NodeUrl,
        ...data,
      },
    });
  }, [activeNetwork, queryClient]);

  return useMutation<
  CombinedEventParams | undefined,
  RequestError,
  CreateTokenAndCollectionProps>(
    async (props) => createTokenAndCollection(
      aptosAccount!,
      aptosClient!,
      props,
    ),
    {
      onSettled: createTokenAndCollectionOnSettled,
    },
  );
};
