import { Network } from "./api-endpoints";

export const DEFAULT_NETWORK = Network.DEVNET;

export enum AptosApiType {
  FULLNODE,
  INDEXER,
  FAUCET,
}
