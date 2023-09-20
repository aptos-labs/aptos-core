import { AptosSettings, ClientConfig } from "../types";
import { NetworkToNodeAPI, NetworkToFaucetAPI, NetworkToIndexerAPI, Network } from "../utils/api-endpoints";
import { AptosApiType, DEFAULT_NETWORK } from "../utils/const";

/**
 * This class holds the config information for the SDK client instance.
 *
 * @public
 */
export class AptosConfig {
  /** The Network that this SDK is associated with. */
  readonly network: Network;

  /**
   * The optional hardcoded fullnode URL to send requests to instead of using the network
   */
  readonly fullnode?: string;

  /**
   * The optional hardcoded faucet URL to send requests to instead of using the network
   */
  readonly faucet?: string;

  /**
   * The optional hardcoded indexer URL to send requests to instead of using the network
   */
  readonly indexer?: string;

  readonly clientConfig?: ClientConfig;

  constructor(settings?: AptosSettings) {
    this.network = settings?.network ?? DEFAULT_NETWORK;
    this.fullnode = settings?.fullnode;
    this.faucet = settings?.faucet;
    this.indexer = settings?.indexer;
    this.clientConfig = settings?.clientConfig ?? {};
  }

  /**
   * Returns the URL endpoint to send the request to.
   * If a custom URL was provided in the config, that URL is returned.
   * If a custom URL was provided but not URL endpoints, an error is thrown.
   * Otherwise, the URL endpoint is derived from the network.
   *
   * @param apiType - The type of Aptos API to get the URL for.
   *
   * @internal
   */
  getRequestUrl(apiType: AptosApiType): string {
    switch (apiType) {
      case AptosApiType.FULLNODE:
        if (this.fullnode !== undefined) return this.fullnode;
        if (this.network === Network.CUSTOM && this.fullnode === undefined)
          throw new Error("Please provide a custom full node url");
        return NetworkToNodeAPI[this.network];
      case AptosApiType.FAUCET:
        if (this.faucet !== undefined) return this.faucet;
        if (this.network === Network.CUSTOM && this.faucet === undefined)
          throw new Error("Please provide a custom faucet url");
        return NetworkToFaucetAPI[this.network];
      case AptosApiType.INDEXER:
        if (this.indexer !== undefined) return this.indexer;
        if (this.network === Network.CUSTOM && this.indexer === undefined)
          throw new Error("Please provide a custom indexer url");
        return NetworkToIndexerAPI[this.network];
      default:
        throw Error(`apiType ${apiType} is not supported`);
    }
  }
}
