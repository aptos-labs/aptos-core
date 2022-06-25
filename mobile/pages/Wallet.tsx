// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useState } from 'react';
import {
  Button, StyleSheet, Text, View,
} from 'react-native';
import { AptosClient, FaucetClient, Types } from 'aptos';
import { RouteProp } from '@react-navigation/native';
import { WalletParams } from './Routes';

const nodeUrl = 'https://fullnode.devnet.aptoslabs.com';
const faucetUrl = 'https://faucet.devnet.aptoslabs.com';

const styles = StyleSheet.create({
  container: {
    alignItems: 'center',
    display: 'flex',
    flex: 1,
    flexDirection: 'column',
    justifyContent: 'center',
    marginLeft: 16,
    marginRight: 16,
  },
});

export default function Wallet({ route }: { route: RouteProp<{ params: WalletParams }> }) {
  const [balance, setBalance] = useState<Number>(0);
  const { address } = route.params;

  const getAccountResources = async () => {
    const client = new AptosClient(nodeUrl);
    return address ? client.getAccountResources(address) : undefined;
  };

  const getAccountBalanceFromAccountResources = (
    accountResources: Types.AccountResource[] | undefined,
  ): Number => {
    if (accountResources) {
      const accountResource = accountResources
        ? accountResources?.find(
          (r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>',
        )
        : undefined;
      const tokenBalance = accountResource
        ? (accountResource.data as { coin: { value: string } }).coin.value
        : undefined;
      return Number(tokenBalance);
    }
    return -1;
  };

  const reload = async () => {
    const resources = await getAccountResources();
    const coins = getAccountBalanceFromAccountResources(resources);
    setBalance(coins);
  };

  const onPress = async () => {
    const faucetClient = new FaucetClient(nodeUrl, faucetUrl);
    await faucetClient.fundAccount(address, 5000);
    await reload();
  };

  reload();

  return (
    <View style={styles.container}>
      <Text>{balance}</Text>
      <Button title="Faucet" onPress={onPress} />
    </View>
  );
}
