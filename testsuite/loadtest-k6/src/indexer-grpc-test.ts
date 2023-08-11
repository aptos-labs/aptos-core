import { sleep } from "k6";
import { Counter } from "k6/metrics";
// see https://k6.io/docs/javascript-api/k6-experimental/grpc/stream/
import grpc from "k6/experimental/grpc";

const GRPC_ADDR = "34.172.237.89:50051";
const GRPC_METHOD = "aptos.indexer.v1.RawData/GetTransactions";
const ledger_version = 32268044;
const count = 2000000;

// relative path from this directory to the proto files
const GRPC_IMPORT_PATHS = ["../../../crates/aptos-protos/proto"];
const GRPC_PROTOS = ["aptos/indexer/v1/raw_data.proto"];


let clients = [];
for (let i = 0; i < 10; i++) {
	const client = new grpc.Client();
	client.load(GRPC_IMPORT_PATHS, ...GRPC_PROTOS);
	clients.push(client);
}

export const options = {
  discardResponseBodies: true,
  scenarios: {
    contacts: {
      executor: "constant-vus",
      vus: 10,
      duration: "100s",
    },
  },
};

const transactionsCounter = new Counter("transactions_count");

// GetTransactions from raw data stream
// Inspiration from: https://github.com/aptos-labs/aptos-indexer-processors/blob/main/typescript/processors/example-write-set-change-processor/processor.ts
export default () => {
  if (__ITER < 10) {
    clients[__ITER].connect(GRPC_ADDR, { plaintext: true });
  }
  let client = clients[__ITER % 10];

  let starting_version = ledger_version - Math.floor(Math.random() * (count)); 
  const req = {
    starting_version: starting_version,
    transactions_count: ledger_version - starting_version,
  };

  const metadata = {
    "x-aptos-data-authorization": "ZFEk3rvVIXclGESJjad4aE8Z03JEEsP27hwxY9Kvw4Kj0T8zZu7PG9RyO6i4GbE9",
  };

  const params = {
    metadata,
  };

  const stream = new grpc.Stream(client, GRPC_METHOD, params);
  stream.write(req);
  // @ts-ignore // to use string event https://k6.io/docs/javascript-api/k6-experimental/grpc/stream/stream-on/
  stream.on("data", (data) => {
    // @ts-ignore
    transactionsCounter.add(data.transactions.length, {index: __ITER});
  });

  // @ts-ignore // to use string event https://k6.io/docs/javascript-api/k6-experimental/grpc/stream/stream-on/
  stream.on("end", () => {
    client.close();
    console.log("All done");
  });

  sleep(0.1);
};
