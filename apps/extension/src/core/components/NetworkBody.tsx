// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
  Flex,
  SimpleGrid,
  Heading,
  useRadioGroup,
} from '@chakra-ui/react';
import React from 'react';
import useGlobalStateContext, { NetworkType } from 'core/hooks/useGlobalState';
import { useQueryClient } from 'react-query';
import NetworkListItem from './NetworkListItem';

export default function NetworkBody() {
  const {
    activeNetworkType,
    networks,
    switchNetwork,
  } = useGlobalStateContext();
  const queryClient = useQueryClient();

  const { getRadioProps, getRootProps } = useRadioGroup({
    defaultValue: activeNetworkType,
    onChange: async (networkType: NetworkType) => {
      await switchNetwork(networkType);
      // Invalidate all queries to clear cached data from previous network
      await queryClient.invalidateQueries();
    },
  });

  return (
    <>
      <SimpleGrid columns={2} width="100%" pb={4}>
        <Flex>
          <Heading fontSize="xl">Network</Heading>
        </Flex>
      </SimpleGrid>
      <VStack mt={2} spacing={2} alignItems="left" {...getRootProps()}>
        {
          Object.keys(networks).map((networkType) => {
            const network = networks[networkType as NetworkType];
            return (
              <NetworkListItem
                key={networkType}
                network={network}
                isLoading={false}
                isDisabled={networkType === NetworkType.LocalHost
                  || networkType === NetworkType.Testnet}
                {...getRadioProps({ value: networkType })}
              />
            );
          })
        }
      </VStack>
    </>
  );
}
