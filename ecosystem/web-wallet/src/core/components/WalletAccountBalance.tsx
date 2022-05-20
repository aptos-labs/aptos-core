import {
  Heading, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import React from 'react';
import { seconaryAddressFontColor } from 'core/components/WalletHeader';
import { getAccountResources, getTestCoinTokenBalanceFromAccountResources } from 'core/queries/account';
import { useQuery } from 'react-query';
import useWalletState from 'core/hooks/useWalletState';
import numeral from 'numeral';

function WalletAccountBalance() {
  const { colorMode } = useColorMode();
  const { aptosAccount } = useWalletState();
  const {
    data: accountResources,
  } = useQuery(
    'getAccountResources',
    () => getAccountResources({ address: aptosAccount?.address() }),
    { refetchInterval: 2000 },
  );

  const tokenBalance = getTestCoinTokenBalanceFromAccountResources({ accountResources });
  const tokenBalanceString = numeral(tokenBalance).format('0,0.0000');

  return (
    <VStack>
      <Text fontSize="sm" color={seconaryAddressFontColor[colorMode]}>Account balance</Text>
      <Heading>{tokenBalanceString}</Heading>
    </VStack>
  );
}

export default WalletAccountBalance;
