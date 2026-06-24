const HtmlWebpackPlugin = require('html-webpack-plugin')

module.exports = {
  mode: 'development',
  module: {
    rules: [
      {
        test: /url\.svg$/,
        use: ['@svgr2/webpack', 'url-loader'],
      },
      {
        test: /simple\.svg$/,
        use: '@svgr2/webpack',
      },
    ],
  },
  plugins: [
    new HtmlWebpackPlugin({
      title: 'Development',
    }),
  ],
}
