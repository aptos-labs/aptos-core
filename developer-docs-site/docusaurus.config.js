// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require('prism-react-renderer/themes/github');
const darkCodeTheme = require('prism-react-renderer/themes/dracula');

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Aptos Labs',
  tagline: 'Developer Documentation',
  url: 'https://docs.aptoslabs.com',
  baseUrl: '/',
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',
  favicon: 'img/favicon.ico',
  organizationName: 'aptos-labs', // Usually your GitHub org/user name.
  projectName: 'developer-docs', // Usually your repo name.

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          routeBasePath: '/',
          sidebarPath: require.resolve('./sidebars.js'),
          // TODO: Please change this to your repo.
          editUrl: 'https://github.com/facebook/docusaurus/tree/main/packages/create-docusaurus/templates/shared/',
        },
        blog: false,
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      navbar: {
        title: 'Developer Documentation',
        logo: {
          alt: 'Aptos Labs Logo',
          src: 'img/aptos_logo_wordmark_transparent_blk.png',
          srcDark: 'img/aptos_logo_wordmark_transparent_white.png',
        },
        items: [
          {
            href: 'TODO: https://github.com/facebook/docusaurus',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Community',
            items: [
              {
                label: 'Reddit',
                href: 'https://todo.com/this',
              },
              {
                label: 'Discord',
                href: 'https://discord.gg/zTDYBEud7U',
              },
              {
                label: 'Twitter',
                href: 'https://twitter.com/aptos_labs',
              },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} Aptos Labs`,
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
      },
    }),
};

module.exports = config;
