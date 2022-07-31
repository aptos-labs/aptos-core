// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
  Flex,
  SimpleGrid,
  Heading,
  useRadioGroup,
} from '@chakra-ui/react';
import React, { useEffect, useState } from 'react';
import { NodeUrl, nodeUrlMap } from 'core/utils/network';
import useWalletState from 'core/hooks/useWalletState';
import { useTestnetStatus } from 'core/queries/network';
import useSwitchNetwork from 'core/mutations/network';
import NetworkListItem from './NetworkListItem';

interface NetworkPreference {
  description?: string;
  title: string;
  value: NodeUrl;
}

const networkPreferences: NetworkPreference[] = Object.entries(nodeUrlMap).map(
  ([key, value]) => ({
    title: key,
    value,
  }),
);

export default function NetworkBody() {
  const {
    nodeUrl,
  } = useWalletState();
  const { data: localTestnetIsLive } = useTestnetStatus();
  const { isLoading, mutateAsync } = useSwitchNetwork();
  const [error, setError] = useState<boolean>(false);

  const onClick = async (event: NodeUrl) => {
    try {
      await mutateAsync({ event, localTestnetIsLive });
    } catch (err) {
      setError(!error);
    }
  };

  const { getRadioProps, getRootProps, setValue: radioSetValue } = useRadioGroup({
    defaultValue: nodeUrl,
    name: 'NodeNetworkUrl',
    onChange: onClick,
  });

  const group = getRootProps();

  useEffect(() => {
    radioSetValue(nodeUrl);
  }, [nodeUrl, error, radioSetValue]);

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
                isDisabled={network.value === nodeUrlMap.Localhost && !localTestnetIsLive}
                isLoading={!isLoading}
                {...radio}
                value={network.value}
              />
            );
          })
        }
      </VStack>
    </>
  );
}
