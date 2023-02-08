import { AptosClient } from "./aptos_client";
import { IndexerClient } from "./indexer_client";

// TODO add graphql codegen generated file
import * as Gen from "./generated/index";

type Network = "mainnet" | "testnet" | "devnet";

type CustomEndpoints = {
  fullnodeUrl: string;
  indexerUrl: string;
};
// TODO move to a const file
const ChainIdToIndexerEndpointMap: Record<string, string> = {
  mainnet: "https://indexer.mainnet.aptoslabs.com/v1/graphql",
  testnet: "https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql",
  devnet: "https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql",
};
// TODO move to a const file
const ChainIdToNodeUrlMap: Record<string, string> = {
  mainnet: "https://fullnode.mainnet.aptoslabs.com/v1",
  testnet: "https://fullnode.testnet.aptoslabs.com/v1",
  devnet: "https://fullnode.devnet.aptoslabs.com/v1",
};

export class Connection {
  aptosClient: AptosClient;
  indexerClient: IndexerClient;

  constructor(
    network?: Network,
    customEndpoints?: CustomEndpoints,
    config?: Partial<Gen.OpenAPIConfig>,
    doNotFixNodeUrl: boolean = false,
  ) {
    let fullNodeUrl = "";
    let indexerUrl = "";

    if (network) {
      fullNodeUrl = ChainIdToNodeUrlMap[network];
      indexerUrl = ChainIdToIndexerEndpointMap[network];
    } else if (customEndpoints) {
      fullNodeUrl = customEndpoints.fullnodeUrl;
      indexerUrl = customEndpoints.indexerUrl;
    } else {
      throw new Error("netowrk and customEndpoints can not be empty.");
    }
    this.aptosClient = new AptosClient(fullNodeUrl, config, doNotFixNodeUrl);
    this.indexerClient = new IndexerClient(indexerUrl);
  }
}

export interface Connection extends AptosClient, IndexerClient {}

/*
In TypeScript, we can’t inherit or extend from more than one class,
Mixins helps us to get around that by creating a partial classes 
that we can combine to form a single class that contains all the methods and properties from the partial classes.
Here, we combine AptosClient and IndexerClient classes into one Connection class that holds all 
methods and properties from both classes.
*/
applyMixins(Connection, [AptosClient, IndexerClient]);

function applyMixins(derivedCtor: any, constructors: any[]) {
  Object.getOwnPropertyNames(AptosClient.prototype).forEach((propertyName) => {
    const propertyDescriptor = Object.getOwnPropertyDescriptor(AptosClient.prototype, propertyName);
    if (!propertyDescriptor) return;
    propertyDescriptor.value = function (...args: any) {
      return (this as any).aptosClient[propertyName](...args);
    };
    Object.defineProperty(Connection.prototype, propertyName, propertyDescriptor);
  });

  Object.getOwnPropertyNames(IndexerClient.prototype).forEach((propertyName) => {
    const propertyDescriptor = Object.getOwnPropertyDescriptor(IndexerClient.prototype, propertyName);
    if (!propertyDescriptor) return;
    propertyDescriptor.value = function (...args: any) {
      return (this as any).indexerClient[propertyName](...args);
    };
    Object.defineProperty(Connection.prototype, propertyName, propertyDescriptor);
  });
}
