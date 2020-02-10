const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin')

module.exports = {
  entry: ['react-hot-loader/patch', './src/index.js'],
  output: {
    filename: 'main.js',
    path: path.resolve(__dirname, 'build'),
  },
  target: 'web',
  devtool: 'inline-source-map',
  devServer: {
    port: 3000,
    contentBase: './build',
    hot: true
  },
  module: {
    rules: [
      {
        test: /\.less$/,
        use: [
          { loader: 'style-loader' },
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
          { loader: 'style-loader' },
          { loader: 'css-loader' },
        ],
      },
      {
        test: /\.m?js$/,
        exclude: /node_modules/,
        use: {
          loader: 'babel-loader'
        }
      },
      {
        test: /\.jsx?$/,
        exclude: /node_modules/,
        use: {
          loader: 'prettier-loader',
        }
      }
    ],
  },
  plugins: [
    new HtmlWebpackPlugin({
      title: 'delta.chat',
      filename: 'index.html',
      template: 'index.html'
    })
  ],
  resolve: {
    alias: {
      'react-dom': '@hot-loader/react-dom',
    }
  }
};

