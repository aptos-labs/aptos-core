// SPDX-FileCopyrightText: 2021 Andrea Pappacoda
//
// SPDX-License-Identifier: Apache-2.0

module.exports = {
  title: 'Pistache',
  tagline: 'An elegant C++ REST framework.',
  url: 'https://pistacheio.github.io',
  baseUrl: '/pistache/',
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',
  favicon: 'img/logo.png',
  organizationName: 'pistacheio', // Usually your GitHub org/user name.
  projectName: 'pistace', // Usually your repo name.
  themeConfig: {
    navbar: {
      title: 'Pistache',
      logo: {
        alt: 'Pistache logo',
        src: 'img/logo.png',
      },
      items: [
        {
          to: 'docs/',
          activeBasePath: 'docs',
          label: 'Docs',
          position: 'left',
        },
        {
          href: 'https://github.com/pistacheio/pistache',
          className: 'header-github-link',
          'aria-label': 'GitHub repository',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {
              label: 'Quickstart',
              to: 'docs/',
            },
            {
              label: 'User guide',
              to: 'docs/http-handler/',
            },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'Stack Overflow',
              href: 'https://stackoverflow.com/questions/tagged/pistache',
            },
            {
              label: '#pistache on Libera.Chat',
              href: 'https://web.libera.chat/#pistache',
            },
          ],
        },
        {
          title: 'More',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/pistacheio/pistache',
            },
          ],
        },
      ],
      copyright: `Pistache, 2015 - ${new Date().getFullYear()}`,
    },
  },
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          editUrl:
            'https://github.com/pistacheio/pistache/edit/master/pistache.io/',
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      },
    ],
  ],
};
