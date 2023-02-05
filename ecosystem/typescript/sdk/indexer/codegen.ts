import type { CodegenConfig } from "@graphql-codegen/cli";

const config: CodegenConfig = {
  overwrite: true,
  schema: [
    "https://cloud.hasura.io/public/graphiql?endpoint=https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql",
  ],
  generates: {
    "src/indexer-generated/graphql.ts": {
      plugins: ["typescript", "typescript-document-nodes"],
    },
    "./graphql.schema.json": {
      plugins: ["introspection"],
    },
  },
};

export default config;
