import { sleep } from "k6";

// see https://k6.io/docs/javascript-api/k6-experimental/grpc/stream/
import grpc from "k6/experimental/grpc";

const GRPC_ADDR = __ENV.GRPC_ADDR || "127.0.0.1:50052";
const GRPC_METHOD = "velor.indexer.v1.RawData/GetTransactions";

// relative path from this directory to the proto files
const GRPC_IMPORT_PATHS = ["../../../crates/velor-protos/proto"];
const GRPC_PROTOS = ["velor/indexer/v1/raw_data.proto"];

const client = new grpc.Client();
client.load(GRPC_IMPORT_PATHS, ...GRPC_PROTOS);

export const options = {
  discardResponseBodies: true,
  scenarios: {
    contacts: {
      executor: "constant-vus",
      vus: 10,
      duration: "30s",
    },
  },
};

// GetTransactions from raw data stream
// Inspiration from: https://github.com/velor-chain/velor-indexer-processors/blob/main/typescript/processors/example-write-set-change-processor/processor.ts
export default () => {
  if (__ITER == 0) {
    client.connect(GRPC_ADDR, { plaintext: true });
  }

  const req = {
    starting_version: 0,
  };

  const metadata = {
    "x-velor-data-authorization": "dummy_token",
  };

  const params = {
    metadata,
  };

  const stream = new grpc.Stream(client, GRPC_METHOD, params);
  stream.write(req);

  // @ts-ignore // to use string event https://k6.io/docs/javascript-api/k6-experimental/grpc/stream/stream-on/
  stream.on("data", (data) => {
    // @ts-ignore
    console.log(`Got ${data.transactions.length} transactions`);
  });

  // @ts-ignore // to use string event https://k6.io/docs/javascript-api/k6-experimental/grpc/stream/stream-on/
  stream.on("end", () => {
    client.close();
    console.log("All done");
  });

  sleep(1);
};
