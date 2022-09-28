// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Center,
  Grid,
  Heading,
  Spinner,
  Text,
  VStack,
  useColorMode,
} from '@chakra-ui/react';
import React, { useMemo } from 'react';
import AvatarImage from 'core/AvatarImage';
import { assetSecondaryBgColor, secondaryBorderColor, secondaryTextColor } from 'core/colors';
import { aptosCoinStoreStructTag } from 'core/constants';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { useAccountCoinResources } from 'core/queries/account';
import { formatCoin } from 'core/utils/coin';
import { AptosLogo } from './AptosLogo';

const CoinType = {
  APTOS_TOKEN: aptosCoinStoreStructTag,
};

function NoAssets() {
  const { colorMode } = useColorMode();
  return (
    <Center
      borderWidth="1px"
      borderRadius=".5rem"
      borderColor={secondaryBorderColor[colorMode]}
      p={4}
    >
      <Text fontSize="md">No assets yet!</Text>
    </Center>
  );
}

interface AssetListItemProps {
  decimals: number;
  name: string;
  symbol: string;
  type: string;
  value: bigint;
}

function AssetListItem({
  decimals,
  name,
  symbol,
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

  const coinInfoName = useMemo(() => {
    switch (type) {
      case CoinType.APTOS_TOKEN: {
        return 'Aptos';
      }
      default: {
        return name;
      }
    }
  }, [type, name]);

  const amount = useMemo(() => {
    switch (type) {
      case CoinType.APTOS_TOKEN:
        return formatCoin(value);
      default:
        return `${formatCoin(value, { decimals, includeUnit: false })} ${symbol}`;
    }
  }, [type, value, decimals, symbol]);

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
          {coinInfoName}
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
  const coinResources = useAccountCoinResources(
    activeAccountAddress,
    {
      refetchInterval: 5000,
    },
  );

  return (
    <VStack px={4} spacing={2} alignItems="stretch">
      <Heading
        py={2}
        fontSize="md"
        color={secondaryTextColor[colorMode]}
      >
        ASSETS
      </Heading>
      {
        coinResources.isLoading
          ? <Spinner alignSelf="center" />
          : undefined
      }
      {
        coinResources.data && coinResources.data.length > 0
          ? (
            <VStack spacing={2}>
              {
                coinResources.data.map((coinResource) => (
                  <AssetListItem key={coinResource.type} {...coinResource} />
                ))
              }
            </VStack>
          )
          : null
      }
      {
        coinResources.data && coinResources.data.length === 0
          ? <NoAssets />
          : null
      }
    </VStack>
  );
}
