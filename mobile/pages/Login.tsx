// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Button, StyleSheet, TextInput, View,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import { StackNavigationProp } from '@react-navigation/stack';
import { StackParamList } from './Routes';

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
  input: {
    alignSelf: 'stretch',
    borderWidth: 1,
    height: 40,
    padding: 10,
  },
});

export default function Login() {
  const navigation = useNavigation<StackNavigationProp<StackParamList>>();
  let key = '';

  const onPress = () => {
    navigation.navigate('Wallet', { address: key });
  };

  const onTextChange = (text: string) => {
    key = text;
  };

  return (
    <View style={styles.container}>
      <TextInput
        style={styles.input}
        placeholder="Private Key..."
        onChangeText={onTextChange}
      />
      <Button title="Submit" onPress={onPress} />
    </View>
  );
}
