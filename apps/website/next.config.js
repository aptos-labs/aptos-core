/** @type {import('next').NextConfig} */

const nextConfig = {
  reactStrictMode: true,

  async redirects() {
    return [
      {
        source: '/docs',
        destination: '/docs/aptos-wallet-intro',
        permanent: true,
      }
    ]
  }
}

module.exports = nextConfig
