// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { FaucetClient } from 'aptos';
import { FAUCET_URL, NODE_URL } from 'core/constants';

export interface FundAccountWithFaucetProps {
  address: string;
  faucetUrl?: string;
  nodeUrl?: string;
}

export const fundAccountWithFaucet = async ({
  nodeUrl = NODE_URL,
  faucetUrl = FAUCET_URL,
  address,
}: FundAccountWithFaucetProps): Promise<void> => {
  const faucetClient = new FaucetClient(nodeUrl, faucetUrl);
  await faucetClient.fundAccount(address, 5000);
};
