# @svgr2/webpack

[![Build Status](https://img.shields.io/travis/gregberge/svgr.svg)](https://travis-ci.org/gregberge/svgr)
[![Version](https://img.shields.io/npm/v/@svgr2/webpack.svg)](https://www.npmjs.com/package/@svgr2/webpack)
[![MIT License](https://img.shields.io/npm/l/@svgr2/webpack.svg)](https://github.com/gregberge/svgr/blob/master/LICENSE)

Webpack loader for SVGR.

```
npm install @svgr2/webpack --save-dev
```

## Usage

In your `webpack.config.js`:

```js
{
  test: /\.svg$/,
  use: ['@svgr2/webpack'],
}
```

In your code:

```js
import Star from './star.svg';

const App = () => (
  <div>
    <Star />
  </div>
);
```

### Passing options

```js
{
  test: /\.svg$/,
  use: [
    {
      loader: '@svgr2/webpack',
      options: {
        native: true,
      },
    },
  ],
}
```

### Using with `url-loader` or `file-loader`

It is possible to use it with [`url-loader`](https://github.com/webpack-contrib/url-loader) or [`file-loader`](https://github.com/webpack-contrib/file-loader).

In your `webpack.config.js`:

```js
{
  test: /\.svg$/,
  use: ['@svgr2/webpack', 'url-loader'],
}
```

In your code:

```js
import starUrl, { ReactComponent as Star } from './star.svg';

const App = () => (
  <div>
    <img src={starUrl} alt="star" />
    <Star />
  </div>
);
```

The named export defaults to `ReactComponent`, but can be customized with the `namedExport` option.

Please note that by default, `@svgr2/webpack` will try to export the React Component via default export if there is no other loader handling svg files with default export. When there is already any other loader using default export for svg files, `@svgr2/webpack` will always export the React component via named export.

If you prefer named export in any case, you may set the `exportType` option to `named`.

### Use your own Babel configuration

By default, `@svgr2/webpack` includes a `babel-loader` with [an optimized configuration](https://github.com/gregberge/svgr/blob/main/packages/webpack/src/index.ts). In some case you may want to apply a custom one (if you are using Preact for an example). You can turn off Babel transformation by specifying `babel: false` in options.

```js
// Example using preact
{
  test: /\.svg$/,
  use: [
    {
      loader: 'babel-loader',
      options: {
        presets: ['preact', 'env'],
      },
    },
    {
      loader: '@svgr2/webpack',
      options: { babel: false },
    }
  ],
}
```

### Handle SVG in CSS, Sass or Less

It is possible to detect the module that requires your SVG using [`Rule.issuer`](https://webpack.js.org/configuration/module/#ruleissuer) in Webpack 5. Using it you can specify two different configurations for JavaScript and the rest of your files.

```js
[
  {
    test: /\.svg(\?v=\d+\.\d+\.\d+)?$/,
    issuer: /\.[jt]sx?$/,
    use: ['babel-loader', '@svgr2/webpack', 'url-loader'],
  },
  {
    test: /\.svg(\?v=\d+\.\d+\.\d+)?$/,
    loader: 'url-loader',
  },
];
```

_[Rule.issuer](https://v4.webpack.js.org/configuration/module/#ruleissuer) in Webpack 4 has additional conditions which are not available in Webpack 5._

## License

MIT
