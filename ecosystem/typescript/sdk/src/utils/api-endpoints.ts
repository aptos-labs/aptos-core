export const NetworkToIndexerAPI: Record<string, string> = {
  mainnet: "https://indexer.mainnet.aptoslabs.com/v1/graphql",
  testnet: "https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql",
  devnet: "https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql",
  local: "http://127.0.0.1:8090/v1/graphql",
};

export const NetworkToNodeAPI: Record<string, string> = {
  mainnet: "https://fullnode.mainnet.aptoslabs.com/v1",
  testnet: "https://fullnode.testnet.aptoslabs.com/v1",
  devnet: "https://fullnode.devnet.aptoslabs.com/v1",
  local: "http://127.0.0.1:8080/v1",
};

export const NodeAPIToNetwork: Record<string, string> = {
  "https://fullnode.mainnet.aptoslabs.com/v1": "mainnet",
  "https://fullnode.testnet.aptoslabs.com/v1": "testnet",
  "https://fullnode.devnet.aptoslabs.com/v1": "devnet",
  "http://127.0.0.1:8080/v1": "local",
};

export enum Network {
  MAINNET = "mainnet",
  TESTNET = "testnet",
  DEVNET = "devnet",
  LOCAL = "local",
}

export interface CustomEndpoints {
  fullnodeUrl: string;
  indexerUrl?: string;
}
