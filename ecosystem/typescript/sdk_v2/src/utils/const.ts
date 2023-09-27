import { Network } from "./api-endpoints";

export const DEFAULT_NETWORK = Network.DEVNET;

export enum AptosApiType {
  FULLNODE,
  INDEXER,
  FAUCET,
}

export const APTOS_COIN = "0x1::aptos_coin::AptosCoin";
