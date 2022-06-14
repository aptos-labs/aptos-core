// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { FaucetClient } from 'aptos';

export interface FundAccountWithFaucetProps {
  address: string;
  faucetUrl: string;
  nodeUrl: string;
}

export const fundAccountWithFaucet = async ({
  address,
  faucetUrl,
  nodeUrl,
}: FundAccountWithFaucetProps): Promise<void> => {
  const faucetClient = new FaucetClient(nodeUrl, faucetUrl);
  await faucetClient.fundAccount(address, 5000);
};
