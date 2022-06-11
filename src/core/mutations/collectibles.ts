// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, TokenClient, RequestError } from 'aptos';
import queryKeys from 'core/queries/queryKeys';
import { AptosAccountState } from 'core/types';
import { AptosNetwork } from 'core/utils/network';
import { useCallback } from 'react';
import { useMutation, useQueryClient } from 'react-query';

interface CreateTokenAndCollectionProps {
  account: AptosAccountState;
  collectionName?: string;
  description?: string;
  name?: string;
  nodeUrl: AptosNetwork;
  supply: number;
  uri?: string;
}

export const defaultRequestErrorAttributes = {
  config: {},
  headers: {},
  status: 400,
  statusText: 'Move abort',
};

export interface RaiseForErrorProps {
  vmStatus: string
}

const raiseForError = ({
  vmStatus,
}: RaiseForErrorProps) => {
  if (vmStatus.includes('Move abort')) {
    throw new RequestError(vmStatus, {
      data: {
        message: vmStatus,
      },
      ...defaultRequestErrorAttributes,
    });
  }
};

export const createTokenAndCollection = async ({
  account,
  collectionName,
  description,
  name,
  nodeUrl,
  supply,
  uri,
}: CreateTokenAndCollectionProps): Promise<void> => {
  if (!account || !(collectionName && description && uri && name)) {
    return;
  }
  const aptosClient = new AptosClient(nodeUrl);
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
  );
  const tokenTxn: any = await aptosClient.getTransaction(tokenTxnHash);
  vmStatus = tokenTxn.vm_status;
  raiseForError({ vmStatus });
};

export const useCreateTokenAndCollection = () => {
  const queryClient = useQueryClient();

  const createTokenAndCollectionOnSettled = useCallback(async () => {
    queryClient.invalidateQueries(queryKeys.getGalleryItems);
    queryClient.invalidateQueries(queryKeys.getAccountResources);
  }, [queryClient]);

  return useMutation<void, RequestError, CreateTokenAndCollectionProps>(createTokenAndCollection, {
    onSettled: createTokenAndCollectionOnSettled,
  });
};
