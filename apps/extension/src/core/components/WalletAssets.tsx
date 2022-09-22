// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { ChevronRightIcon } from '@chakra-ui/icons';
import {
  Box,
  Button,
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
import ChakraLink from 'core/components/ChakraLink';
import { aptosCoinStoreStructTag } from 'core/constants';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { useAccountCoinResources } from 'core/queries/account';
import useActivity from 'core/queries/useActivity';
import { formatCoin } from 'core/utils/coin';
import { ActivityList } from './ActivityList';
import { AptosLogo } from './AptosLogo';
import { Routes } from '../routes';

const CoinType = {
  APTOS_TOKEN: aptosCoinStoreStructTag,
};

function NoAssets() {
  const { colorMode } = useColorMode();
  return (
    <Box w="100%" borderWidth="1px" borderRadius=".5rem" borderColor={secondaryBorderColor[colorMode]}>
      <Center height="100%" p={4}>
        <Text fontSize="md" textAlign="center">No assets yet!</Text>
      </Center>
    </Box>
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
  const { data: coinResources } = useAccountCoinResources(
    activeAccountAddress,
    {
      refetchInterval: 5000,
    },
  );

  const activity = useActivity({ pageSize: 5 });
  const transactions = useMemo(() => activity.data?.pages[0]?.txns, [activity.data]);

  return (
    <VStack width="100%" alignItems="flex-start" px={4}>
      <Heading
        py={2}
        fontSize="md"
        color={secondaryTextColor[colorMode]}
      >
        ASSETS
      </Heading>
      <VStack width="100%" gap={1}>
        {
          (coinResources && coinResources.length > 0) ? (
            coinResources?.map((coinResource) => (
              <AssetListItem key={coinResource.type} {...coinResource} />
            ))
          ) : <NoAssets />
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
      <VStack w="100%" spacing={0}>
        {
          activity.isLoading
            ? <Spinner />
            : <ActivityList transactions={transactions} />
        }
        {
          activity.hasNextPage
            ? (
              <ChakraLink key="View more" w="100%" to={Routes.activity.path}>
                <Button
                  mt={3}
                  py={6}
                  width="100%"
                  rightIcon={<ChevronRightIcon />}
                  justifyContent="space-between"
                >
                  View more
                </Button>
              </ChakraLink>
            )
            : null
        }
      </VStack>
    </VStack>
  );
}
