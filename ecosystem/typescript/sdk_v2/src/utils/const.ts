import { Network } from "./api-endpoints";

export const DEFAULT_NETWORK = Network.DEVNET;
export const DEFAULT_TXN_TIMEOUT_SEC = 20;

export enum AptosApiType {
  FULLNODE,
  INDEXER,
  FAUCET,
}

export const DEFAULT_MAX_GAS_AMOUNT = 200000;
// Transaction expire timestamp
export const DEFAULT_TXN_EXP_SEC_FROM_NOW = 20;
