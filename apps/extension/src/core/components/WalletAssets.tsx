// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Center,
  Grid, Heading, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import AvatarImage from 'core/AvatarImage';
import { assetSecondaryBgColor, secondaryTextColor } from 'core/colors';
import { aptosCoinStoreStructTag } from 'core/constants';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { useAccountCoinResources } from 'core/queries/account';
import { formatCoin } from 'core/utils/coin';
import React, { useMemo } from 'react';
import { AptosLogo } from './AptosLogo';
import { TransactionList } from './TransactionList';

const CoinType = {
  APTOS_TOKEN: aptosCoinStoreStructTag,
};

interface AssetListItemProps {
  type: string;
  value: number;
}

function AssetListItem({
  type,
  value,
}: AssetListItemProps) {
  const { colorMode } = useColorMode();

  // TODO: Will need to cache some logos and symbols for relevent
  // coins since they don't appear in account resources
  const logo = useMemo(() => {
    switch (type) {
      case CoinType.APTOS_TOKEN:
        return <AptosLogo />;
      default:
        return <AvatarImage size={32} address={type} />;
    }
  }, [type]);

  const name = useMemo(() => {
    switch (type) {
      case CoinType.APTOS_TOKEN: {
        return 'Aptos';
      }
      default: {
        const splitName = type.split('::')[4].replace('>', '');
        return splitName;
      }
    }
  }, [type]);

  const amount = useMemo(() => {
    switch (type) {
      case CoinType.APTOS_TOKEN:
        return formatCoin(value);
      default:
        return `${value}`;
    }
  }, [type, value]);

  return (
    <Grid
      templateColumns="32px 1fr 90px"
      width="100%"
      gap={4}
      p={4}
      borderRadius="0.5rem"
      bgColor={assetSecondaryBgColor[colorMode]}
    >
      <Center width="100%" height="100%">
        {logo}
      </Center>
      <VStack fontSize="md" alignItems="left" spacing={0}>
        <Text fontWeight={600}>
          {name}
        </Text>
        <Text color={secondaryTextColor[colorMode]}>
          {amount}
        </Text>
      </VStack>
      <Box />
    </Grid>
  );
}

export default function WalletAssets() {
  const { colorMode } = useColorMode();
  const { activeAccountAddress } = useActiveAccount();
  const { data: coinResources } = useAccountCoinResources(
    activeAccountAddress,
    {
      refetchInterval: 5000,
    },
  );

  return (
    <VStack width="100%" alignItems="flex-start" px={4}>
      <Heading
        py={2}
        fontSize="md"
        color={secondaryTextColor[colorMode]}
      >
        ASSETS
      </Heading>
      <VStack width="100%" gap={2}>
        {
          coinResources?.map((coinResource) => (
            <AssetListItem key={coinResource.type} {...coinResource} />
          ))
        }
      </VStack>
      <Heading
        py={4}
        pb={2}
        fontSize="md"
        color={secondaryTextColor[colorMode]}
      >
        RECENT TRANSACTIONS
      </Heading>
      <TransactionList limit={5} />
    </VStack>
  );
}
