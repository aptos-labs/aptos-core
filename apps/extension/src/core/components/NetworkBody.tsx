// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
  Box,
  useRadioGroup,
  Button,
  useColorMode,
} from '@chakra-ui/react';
import React from 'react';
import { useNetworks } from 'core/hooks/useNetworks';
import { useQueryClient } from 'react-query';
import Routes from 'core/routes';
import { buttonBorderColor, customColors, secondaryButtonBgColor } from 'core/colors';
import { AddIcon } from '@chakra-ui/icons';
import { switchNetworkToast } from 'core/components/Toast';
import { useNavigate } from 'react-router-dom';
import NetworkListItem from './NetworkListItem';

export default function NetworkBody() {
  const {
    activeNetworkName,
    networks,
    removeNetwork,
    switchNetwork,
  } = useNetworks();
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const { colorMode } = useColorMode();

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
    <VStack display="flex" height="100%" width="100%">
      <VStack overflowY="auto" mt={2} spacing={2} alignItems="left" {...getRootProps()} flex={1} height="100%">
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

      <Box width="100%" borderTop="1px" pt={4} px={4} borderColor={buttonBorderColor[colorMode]}>
        <Button
          width="100%"
          size="sm"
          height="48px"
          border="1px"
          bgColor={secondaryButtonBgColor[colorMode]}
          borderColor={customColors.navy[200]}
          onClick={() => navigate(Routes.addNetwork.path)}
          leftIcon={<AddIcon />}
        >
          Add
        </Button>
      </Box>
    </VStack>
  );
}
