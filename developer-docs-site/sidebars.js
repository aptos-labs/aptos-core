/**
 * Creating a sidebar enables you to:
 - create an ordered group of docs
 - render a sidebar for each doc of that group
 - provide next/previous navigation

 The sidebars can be generated from the filesystem, or explicitly defined here.

 Create as many sidebars as you want.
 */

// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  // By default, Docusaurus generates a sidebar from the docs folder structure
  // defaultSidebar: [{type: 'autogenerated', dirName: '.', }],
  aptosSidebar: [
    "index",
    "whats-new-in-docs",
    "guides/getting-started",
    {
      type: "category",
      label: "Aptos Quickstarts",
      link: { type: "doc", id: "tutorials/index" },
      collapsible: true,
      collapsed: true,
      items: [
        "tutorials/first-transaction",
        "tutorials/first-move-module",
        "tutorials/first-dapp",
        "tutorials/first-coin",
        "tutorials/your-first-nft",
      ],
    },
    {
      type: "category",
      label: "Concepts",
      link: { type: "doc", id: "concepts/index" },
      collapsible: true,
      collapsed: true,
      items: [
        "concepts/basics-txns-states",
        "concepts/basics-accounts",
        "concepts/basics-events",
        "concepts/basics-gas-txn-fee",
        "concepts/basics-merkle-proof",
        "concepts/basics-fullnodes",
        "concepts/basics-validator-nodes",
        "concepts/basics-node-networks-sync",
      ],
    },
    {
      type: "category",
      label: "Guides",
      link: { type: "doc", id: "guides/index" },
      collapsible: true,
      collapsed: true,
      items: [
        "guides/basics-life-of-txn",
        "guides/sign-a-transaction",
        "guides/move-guides/move-on-aptos",
        "guides/interacting-with-the-blockchain",
        "guides/building-wallet-extension",
        "guides/guide-for-system-integrators",
      ],
    },
    {
      type: "category",
      label: "Nodes",
      link: { type: "doc", id: "nodes/index" },
      collapsible: true,
      collapsed: true,
      items: [
        "nodes/aptos-deployments",
        {
          type: "category",
          label: "AIT-2",
          link: { type: "doc", id: "nodes/ait/index" },
          collapsible: true,
          collapsed: true,
          items: [
            "nodes/ait/node-requirements",
            "nodes/ait/node-liveness-criteria",
            "nodes/ait/connect-to-testnet",
            "nodes/ait/additional-doc",
          ],
        },
        {
          type: "category",
          label: "Validators",
          link: { type: "doc", id: "nodes/validator-node/index" },
          collapsible: true,
          collapsed: true,
          items: [
            "nodes/validator-node/using-aws",
            "nodes/validator-node/using-azure",
            "nodes/validator-node/using-gcp",
            "nodes/validator-node/using-docker",
            "nodes/validator-node/using-source-code",
          ],
        },
        {
          type: "category",
          label: "FullNode for Devnet",
          link: { type: "doc", id: "nodes/full-node/index" },
          collapsible: true,
          collapsed: true,
          items: [
            "nodes/full-node/fullnode-source-code-and-docker",
            "nodes/full-node/update-fullnode-with-new-releases",
            "nodes/full-node/network-identity-fullnode",
            "nodes/full-node/troubleshooting-fullnode",
            "nodes/full-node/run-a-fullnode-on-gcp",
          ],
        },
        "nodes/run-a-local-testnet",
        "nodes/node-health-checker",
        "nodes/node-health-checker-faq",
        "nodes/staking",
      ],
    },
    {
      type: "category",
      label: "SDKs",
      collapsible: true,
      collapsed: true,
      items: [
        {
          type: "link",
          label: "Typescript SDK",
          href: "https://aptos-labs.github.io/ts-sdk-doc/",
        },
        "sdks/aptos-sdk-overview",
        "sdks/transactions-with-ts-sdk",
        "sdks/python-sdk",
      ],
    },
    {
      type: "category",
      label: "Aptos CLI",
      collapsible: true,
      collapsed: true,
      link: { type: "doc", id: "cli-tools/aptos-cli-tool/index" },
      items: ["cli-tools/aptos-cli-tool/install-aptos-cli", "cli-tools/aptos-cli-tool/use-aptos-cli"],
    },
    {
      type: "category",
      label: "API",
      collapsible: true,
      collapsed: true,
      link: { type: "doc", id: "api/index" },
      items: ["api/index"],
    },
    "reference/telemetry",
    "reference/glossary",
  ],
};

module.exports = sidebars;
