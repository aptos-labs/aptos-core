import { ClientConfig } from "../client/types";
import { NetworkToNodeAPI, NetworkToFaucetAPI, NetworkToIndexerAPI, Network } from "../utils/api-endpoints";
import { DEFAULT_NETWORK } from "../utils/const";

export class AptosConfig {
  readonly network?: Network;

  readonly fullnode?: string;

  readonly faucet?: string;

  readonly indexer?: string;

  readonly clientConfig?: ClientConfig;

  constructor(config?: AptosConfig) {
    this.network = config?.network ?? DEFAULT_NETWORK;
    this.fullnode = config?.fullnode ?? NetworkToNodeAPI[this.network];
    this.faucet = config?.faucet ?? NetworkToFaucetAPI[this.network];
    this.indexer = config?.indexer ?? NetworkToIndexerAPI[this.network];
    this.clientConfig = config?.clientConfig ?? {};
  }
}
