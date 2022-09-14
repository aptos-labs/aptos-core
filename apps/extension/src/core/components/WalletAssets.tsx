// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Grid, Heading, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import { assetSecondaryBgColor, secondaryTextColor } from 'core/colors';
import { aptosCoinStoreStructTag } from 'core/constants';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { useAccountCoinResources } from 'core/queries/account';
import { formatCoin } from 'core/utils/coin';
import React, { useMemo } from 'react';
import { AptosLogo } from './AptosLogo';

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
  const logo = useMemo(() => {
    switch (type) {
      case CoinType.APTOS_TOKEN:
        return <AptosLogo />;
      default:
        return <VStack>No logo</VStack>;
    }
  }, [type]);

  const name = useMemo(() => {
    switch (type) {
      case CoinType.APTOS_TOKEN:
        return 'Aptos';
      default:
        return 'No name';
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
      templateColumns="45px 1fr 90px"
      width="100%"
      gap={4}
      p={4}
      borderRadius="0.5rem"
      bgColor={assetSecondaryBgColor[colorMode]}
    >
      <Box>
        {logo}
      </Box>
      <VStack alignItems="left" spacing={0}>
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
  );

  return (
    <VStack width="100%" alignItems="flex-start" px={4}>
      <Heading
        py={4}
        pb={2}
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
    </VStack>
  );
}
