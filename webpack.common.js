const path = require('path');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin')
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');

module.exports = {
  entry: './client.js',
  output: {
    filename: 'main.js',
    path: path.resolve(__dirname, 'build'),
  },
  target: 'web',
  module: {
    rules: [
      {
        test: /\.less$/,
        use: [
          {
            loader: MiniCssExtractPlugin.loader,
            options: {
              publicPath: '/',
              esModule: true,
              hmr: process.env.NODE_ENV === 'development'
            },
          },
          { loader: 'css-loader' },
          {
            loader: 'less-loader',
            options: {
              paths: [path.resolve(__dirname, 'node_modules')],
            },
          },
        ],
      },
      {
        test: /\.css$/,
        use: [
          {
            loader: MiniCssExtractPlugin.loader,
            options: {
              publicPath: '/',
              esModule: true,
              hmr: process.env.NODE_ENV === 'development'
            },
          },
          { loader: 'css-loader' },
        ],
      },
    ],
  },
  plugins: [
    new HtmlWebpackPlugin({
      title: 'delta.chat',
      filename: 'index.html',
      template: 'index.html'
    }),
     new MiniCssExtractPlugin({
      filename: '[name].css',
      chunkFilename: '[id].css',
     }),
    new WasmPackPlugin({
      crateDirectory: ".",
      extraArgs: "--no-typescript",
    })
  ],
};

