const path = require('path');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin')

module.exports = {
  entry: ['./src/index.js'],
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
      {
        test: /\.m?js$/,
        exclude: [
          /node_modules/,
          path.join(__dirname, 'src/electron.js'),
          path.join(__dirname, 'src/preload.js'),
        ],
        use: {
          loader: 'babel-loader'
        }
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
  ],
};

