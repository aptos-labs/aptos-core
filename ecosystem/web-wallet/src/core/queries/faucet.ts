// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { FaucetClient } from 'aptos';
import { LOCAL_FAUCET_URL, LOCAL_NODE_URL } from 'core/constants';

export interface FundAccountWithFaucetProps {
  address: string;
  faucetUrl?: string;
  nodeUrl?: string;
}

export const fundAccountWithFaucet = async ({
  nodeUrl = LOCAL_NODE_URL,
  faucetUrl = LOCAL_FAUCET_URL,
  address,
}: FundAccountWithFaucetProps): Promise<void> => {
  const faucetClient = new FaucetClient(nodeUrl, faucetUrl);
  await faucetClient.fundAccount(address, 5000);
};
