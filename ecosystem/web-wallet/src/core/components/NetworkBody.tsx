// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
  Flex,
  SimpleGrid,
  Heading,
  useRadioGroup,
  useToast,
} from '@chakra-ui/react';
import React, { useEffect, useState } from 'react';
import { AptosNetwork } from 'core/utils/network';
import { DEVNET_NODE_URL, LOCAL_NODE_URL } from 'core/constants';
import useWalletState from 'core/hooks/useWalletState';
import { useQuery } from 'react-query';
import { getLocalhostIsLive } from 'core/queries/network';
import useSwitchNetwork from 'core/mutations/network';
import NetworkListItem from './NetworkListItem';

interface NetworkPreference {
  description?: string;
  title?: 'Devnet' | 'Localhost';
  value: AptosNetwork;
}

const networkPreferences: NetworkPreference[] = [
  {
    title: 'Devnet',
    value: DEVNET_NODE_URL,
  },
  {
    title: 'Localhost',
    value: LOCAL_NODE_URL,
  },
];

export default function NetworkBody() {
  const {
    aptosNetwork,
  } = useWalletState();
  const { data: localTestnetIsLive } = useQuery('getTestnetStatus', getLocalhostIsLive, { refetchInterval: 1000 });
  const { isLoading, mutateAsync } = useSwitchNetwork();
  const [error, setError] = useState<boolean>(false);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const toast = useToast();

  const onClick = async (event: AptosNetwork) => {
    try {
      await mutateAsync({ event, localTestnetIsLive });
    } catch (err) {
      setError(!error);
    }
  };

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { getRadioProps, getRootProps, setValue: radioSetValue } = useRadioGroup({
    defaultValue: aptosNetwork,
    name: 'aptosNetwork',
    onChange: onClick,
  });

  const group = getRootProps();

  useEffect(() => {
    radioSetValue(aptosNetwork);
  }, [error]);

  return (
    <>
      <SimpleGrid columns={2} width="100%" pb={4}>
        <Flex>
          <Heading fontSize="xl">Network</Heading>
        </Flex>
      </SimpleGrid>
      <VStack mt={2} spacing={2} alignItems="left" {...group}>
        {
          networkPreferences.map((network) => {
            const radio = getRadioProps({ value: network.value });
            return (
              <NetworkListItem
                key={network.value}
                isDisabled={network.value === LOCAL_NODE_URL && !localTestnetIsLive}
                isLoading={!isLoading}
                {...radio}
              />
            );
          })
        }
      </VStack>
    </>
  );
}
