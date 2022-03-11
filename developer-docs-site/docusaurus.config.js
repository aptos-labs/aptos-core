// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require("prism-react-renderer/themes/github");
const darkCodeTheme = require("prism-react-renderer/themes/dracula");

const codeInjector = require("./src/remark/code-injector");

/** @type {import("@docusaurus/types").Config} */
const config = {
  title: "Aptos Labs",
  tagline: "Developer Documentation",
  url: "https://docs.aptoslabs.com",
  baseUrl: "/",
  onBrokenLinks: "throw",
  onBrokenMarkdownLinks: "warn",
  favicon: "img/favicon.ico",
  organizationName: "aptos-labs", // Usually your GitHub org/user name.
  projectName: "developer-docs", // Usually your repo name.

  presets: [
    [
      "classic",
      /** @type {import("@docusaurus/preset-classic").Options} */
      ({
        docs: {
          routeBasePath: "/",
          sidebarPath: require.resolve("./sidebars.js"),
          editUrl: "https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/",
          remarkPlugins: [codeInjector],
        },
        blog: false,
        theme: {
          customCss: require.resolve("./src/css/custom.css"),
        },
        gtag: {
          trackingID: "G-HVB7QFB9PQ",
        },
      }),
    ],
  ],

  themeConfig:
  /** @type {import("@docusaurus/preset-classic").ThemeConfig} */
    ({
      navbar: {
        title: "Developer Documentation",
        logo: {
          alt: "Aptos Labs Logo",
          src: "img/aptos_logo_wordmark_transparent_blk.png",
          srcDark: "img/aptos_logo_wordmark_transparent_white.png",
        },
        items: [
          {
            href: "https://github.com/aptos-labs/aptos-core/",
            label: "GitHub",
            position: "right",
          },
        ],
      },
      footer: {
        style: "dark",
        links: [
          {
            title: "Community",
            items: [
              {
                label: "Reddit",
                href: "https://www.reddit.com/r/aptoslabs/",
              },
              {
                label: "Discord",
                href: "https://discord.gg/zTDYBEud7U",
              },
              {
                label: "Twitter",
                href: "https://twitter.com/aptoslabs",
              },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} Aptos Labs`,
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
        additionalLanguages: ["rust"],
      },
    }),
};

module.exports = config;
