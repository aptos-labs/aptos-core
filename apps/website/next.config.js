/** @type {import('next').NextConfig} */

const nextConfig = {
  reactStrictMode: true,

  async redirects() {
    return [
      {
        source: '/docs',
        destination: '/docs/petra-intro',
        permanent: true,
      }
    ]
  }
}

module.exports = nextConfig
