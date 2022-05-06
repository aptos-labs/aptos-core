const webpack = require('webpack')

module.exports = {
  resolve: {
    extensions: ['.ts', '.tsx', '.js', '.json'],
    fallback: {
      buffer: require.resolve('buffer')
    }
  },
  plugins: [
    new webpack.ProvidePlugin({
      Buffer: ['buffer', 'Buffer']
    })
  ],
  module: {
    rules: [
      { test: /\.tsx?$/, loader: 'ts-loader' }
    ]
  }
}
