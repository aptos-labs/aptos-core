import { Network } from "./api-endpoints";

export const DEFAULT_NETWORK = Network.DEVNET;
export const DEFAULT_TXN_TIMEOUT_SEC = 20;

export enum AptosApiType {
  FULLNODE,
  INDEXER,
  FAUCET,
}
