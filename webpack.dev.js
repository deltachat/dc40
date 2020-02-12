const merge = require('webpack-merge');
const common = require('./webpack.common.js');

module.exports = merge(common, {
  entry: ['react-hot-loader/patch'],
  mode: 'development',
  devtool: 'inline-source-map',
  devServer: {
    port: 3000,
    contentBase: './build',
    hot: true
  },
  module: {
    rules: [
      {
        test: /\.jsx?$/,
        exclude: /node_modules/,
        use: {
          loader: 'prettier-loader',
        }
      }
    ]
  },
  resolve: {
    alias: {
      'react-dom': '@hot-loader/react-dom',
    }
  }
});
