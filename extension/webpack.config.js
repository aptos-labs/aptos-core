const path = require('path')
const webpack = require('webpack')
const TsconfigPathsPlugin = require('tsconfig-paths-webpack-plugin');

module.exports = {
  resolve: {
    extensions: ['.ts', '.tsx', '.js', '.jsx', '.json'],
    fallback: {
      buffer: require.resolve('buffer')
    },
    plugins: [new TsconfigPathsPlugin({
      configFile: 'tsconfig.json'
    })],
  },
  plugins: [
    new webpack.ProvidePlugin({
      Buffer: ['buffer', 'Buffer']
    })
  ],
  module: {
    rules: [
      { test: /\.tsx?$/, loader: 'ts-loader' },
      {
        test: /\.jsx?$/,
        use: [
          {
              loader: 'babel-loader',
              options: {
                presets: [
                  '@babel/preset-env',
                  '@babel/preset-react',
                ]
              }
          }],
        exclude: /node_modules/
      }
    ]
  }
}
