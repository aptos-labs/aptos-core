// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useQuery, useQueryClient, UseQueryOptions } from 'react-query';
import { useNetworks } from 'core/hooks/useNetworks';
import { Resource } from 'core/types/resource';

export const getAccountResourcesQueryKey = 'getAccountResources';

const defaultQueryOptions = {
  staleTime: 3000,
};

/**
 * Function for manually fetching account resources.
 * Leverages react-query caching mechanisms and shares data with `useAccountResources` query
 * @param address account address
 */
export function useFetchAccountResources(address: string) {
  const { aptosClient } = useNetworks();
  const queryClient = useQueryClient();

  return () => queryClient.fetchQuery<Resource[]>(
    [getAccountResourcesQueryKey, address],
    async () => aptosClient.getAccountResources(address) as Promise<Resource[]>,
    defaultQueryOptions,
  );
}

/**
 * Query for retrieving account resources
 * @param address account address
 * @param options query options
 */
export function useAccountResources(
  address: string | undefined,
  options?: UseQueryOptions<Resource[] | undefined>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<Resource[] | undefined>(
    [getAccountResourcesQueryKey, address],
    async () => (address !== undefined
      ? aptosClient.getAccountResources(address) as Promise<Resource[]>
      : undefined),
    {
      ...defaultQueryOptions,
      ...options,
    },
  );
}
