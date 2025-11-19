/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  transpilePackages: ['@skill-gaming/sdk'],
  webpack: (config) => {
    config.resolve.fallback = {
      ...config.resolve.fallback,
      fs: false,
      process: false,
      buffer: require.resolve('buffer/'),
    };
    return config;
  },
};

module.exports = nextConfig;
