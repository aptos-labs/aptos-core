import { AptosClient } from "./aptos_client";
import { IndexerClient } from "./indexer";

import { CustomEndpoints, Network, NetworkToIndexerAPI, NetworkToNodeAPI } from "../utils";
import { ClientConfig } from "../client";

type NetworkWithCustom = Network | "CUSTOM";
/**
 * Builds a Provider class with an aptos client configured to connect to an Aptos node
 * and indexer client configured to connect to Aptos Indexer.
 *
 * It creates AptosClient and IndexerClient instances based on the network or custom endpoints provided.
 *
 * This class holds both AptosClient and IndexerClient classes's methods and properties so we
 * can instantiate the Provider class and use it to query full node and/or Indexer.
 *
 * NOTE: Indexer client can be undefined/not set when we use Network.LOCAL (since Indexer
 * does not support local environment) or when we use a CUSTOM network to support applications
 * that only use custom fullnode and not Indexer
 *
 * @example An example of how to use this class with a live network
 * ```
 * const provider = new Provider(Network.DEVNET)
 * const account = await provider.getAccount("0x123");
 * const accountTokens = await provider.getOwnedTokens("0x123");
 * ```
 *
 * @example An example of how to use this class with a local network. Indexer
 * doesn't support local network.
 * ```
 * const provider = new Provider(Network.LOCAL)
 * const account = await provider.getAccount("0x123");
 * ```
 *
 * @example An example of how to use this class with a custom network.
 * ```
 * const provider = new Provider({fullnodeUrl:"my-fullnode-url",indexerUrl:"my-indexer-url"})
 * const account = await provider.getAccount("0x123");
 * const accountTokens = await provider.getOwnedTokens("0x123");
 * ```
 *
 * @param network enum of type Network - MAINNET | TESTNET | DEVNET | LOCAL or custom endpoints of type CustomEndpoints
 * @param config optional ClientConfig config arg - additional configuration we can pass with the request to the server.
 */
export class Provider {
  aptosClient: AptosClient;

  indexerClient?: IndexerClient;

  network: NetworkWithCustom;

  constructor(network: Network | CustomEndpoints, config?: ClientConfig, doNotFixNodeUrl: boolean = false) {
    let fullNodeUrl = null;
    let indexerUrl = null;

    if (typeof network === "object" && isCustomEndpoints(network)) {
      fullNodeUrl = network.fullnodeUrl;
      indexerUrl = network.indexerUrl;
      this.network = "CUSTOM";
    } else {
      fullNodeUrl = NetworkToNodeAPI[network];
      indexerUrl = NetworkToIndexerAPI[network];
      this.network = network;
    }

    if (this.network === "CUSTOM" && !fullNodeUrl) {
      throw new Error("fullnode url is not provided");
    }

    if (indexerUrl) {
      this.indexerClient = new IndexerClient(indexerUrl, config);
    }
    this.aptosClient = new AptosClient(fullNodeUrl, config, doNotFixNodeUrl);
  }
}

export interface Provider extends AptosClient, IndexerClient {}

/**
In TypeScript, we can’t inherit or extend from more than one class,
Mixins helps us to get around that by creating a partial classes 
that we can combine to form a single class that contains all the methods and properties from the partial classes.
{@link https://www.typescriptlang.org/docs/handbook/mixins.html#alternative-pattern}

Here, we combine AptosClient and IndexerClient classes into one Provider class that holds all 
methods and properties from both classes.
*/
function applyMixin(targetClass: any, baseClass: any, baseClassProp: string) {
  // Mixin instance methods
  Object.getOwnPropertyNames(baseClass.prototype).forEach((propertyName) => {
    const propertyDescriptor = Object.getOwnPropertyDescriptor(baseClass.prototype, propertyName);
    if (!propertyDescriptor) return;
    // eslint-disable-next-line func-names
    propertyDescriptor.value = function (...args: any) {
      return (this as any)[baseClassProp][propertyName](...args);
    };
    Object.defineProperty(targetClass.prototype, propertyName, propertyDescriptor);
  });
  // Mixin static methods
  Object.getOwnPropertyNames(baseClass).forEach((propertyName) => {
    const propertyDescriptor = Object.getOwnPropertyDescriptor(baseClass, propertyName);
    if (!propertyDescriptor) return;
    // eslint-disable-next-line func-names
    propertyDescriptor.value = function (...args: any) {
      return (this as any)[baseClassProp][propertyName](...args);
    };
    if (targetClass.hasOwnProperty.call(targetClass, propertyName)) {
      // The mixin has already been applied, so skip applying it again
      return;
    }
    Object.defineProperty(targetClass, propertyName, propertyDescriptor);
  });
}

applyMixin(Provider, AptosClient, "aptosClient");
applyMixin(Provider, IndexerClient, "indexerClient");

// use exhaustive type predicates
function isCustomEndpoints(network: CustomEndpoints): network is CustomEndpoints {
  return network.fullnodeUrl !== undefined && typeof network.fullnodeUrl === "string";
}
