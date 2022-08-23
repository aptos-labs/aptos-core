// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
  HStack,
  useRadioGroup,
  Button,
} from '@chakra-ui/react';
import React from 'react';
import useGlobalStateContext from 'core/hooks/useGlobalState';
import { useQueryClient } from 'react-query';
import Routes from 'core/routes';
import { AddIcon } from '@chakra-ui/icons';
import ChakraLink from 'core/components/ChakraLink';
import { switchNetworkToast } from 'core/components/Toast';
import NetworkListItem from './NetworkListItem';

export default function NetworkBody() {
  const {
    activeNetworkName,
    networks,
    removeNetwork,
    switchNetwork,
  } = useGlobalStateContext();
  const queryClient = useQueryClient();

  const onSwitchNetwork = async (networkName: string) => {
    await switchNetwork(networkName);
    // Invalidate all queries to clear cached data from previous network
    await queryClient.invalidateQueries();
  };

  const { getRadioProps, getRootProps, setValue } = useRadioGroup({
    defaultValue: activeNetworkName,
    onChange: onSwitchNetwork,
  });

  const onRemoveNetwork = async (networkName: string) => {
    await removeNetwork(networkName);

    if (networkName === activeNetworkName) {
      const firstAvailableNetworkName = Object.keys(networks!).filter((n) => n !== networkName)[0];
      switchNetworkToast(firstAvailableNetworkName, true);
      setValue(firstAvailableNetworkName);
    } else if (activeNetworkName) {
      switchNetworkToast(activeNetworkName!, false);
    }
  };

  return (
    <>
      <HStack justifyContent="end">
        <ChakraLink to={Routes.addNetwork.path}>
          <Button
            colorScheme="teal"
            size="sm"
            leftIcon={<AddIcon />}
          >
            Add
          </Button>
        </ChakraLink>
      </HStack>
      <VStack mt={2} spacing={2} alignItems="left" {...getRootProps()}>
        {
          networks ? Object.keys(networks).map((networkName) => (
            <NetworkListItem
              key={networkName}
              network={networks[networkName]}
              {...getRadioProps({ value: networkName })}
              onRemove={onRemoveNetwork}
            />
          )) : null
        }
      </VStack>
    </>
  );
}
