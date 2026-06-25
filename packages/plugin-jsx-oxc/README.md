# @svgr2/plugin-jsx-oxc

[![Build Status](https://img.shields.io/travis/smooth-code/svgr.svg)](https://travis-ci.org/smooth-code/svgr)
[![Version](https://img.shields.io/npm/v/@svgr2/plugin-jsx.svg)](https://www.npmjs.com/package/@svgr2/plugin-jsx)
[![MIT License](https://img.shields.io/npm/l/@svgr2/plugin-jsx.svg)](https://github.com/smooth-code/svgr/blob/master/LICENSE)

Transforms SVG into JSX.

## Install

```
npm install --save-dev @svgr2/plugin-jsx
```

## Usage

**.svgrrc**

```json
{
  "plugins": ["@svgr2/plugin-jsx"]
}
```

## How does it work?

`@svgr2/plugin-jsx` consists in three phases:

- Parsing the SVG code using [svg-parser](https://github.com/Rich-Harris/svg-parser)
- Converting the [HAST](https://github.com/syntax-tree/hast) into a [Babel AST](https://github.com/babel/babel/blob/master/packages/babel-parser/ast/spec.md)
- Applying [`@svgr2/babel-preset`](../babel-preset/README.md) transformations

## Applying custom transformations

You can extend the Babel config applied in this plugin using `jsx.babelConfig` config path:

```js
// .svgrrc.js

module.exports = {
  jsx: {
    babelConfig: {
      plugins: [
        // For an example, this plugin will remove "id" attribute from "svg" tag
        [
          '@svgr2/babel-plugin-remove-jsx-attribute',
          {
            elements: ['svg'],
            attributes: ['id'],
          },
        ],
      ],
    },
  },
};
```

Several Babel plugins are available:

- [`@svgr2/babel-plugin-add-jsx-attribute`](../babel-plugin-add-jsx-attribute/README.md)
- [`@svgr2/babel-plugin-remove-jsx-attribute`](../babel-plugin-remove-jsx-attribute/README.md)
- [`@svgr2/babel-plugin-remove-jsx-empty-expression`](../babel-plugin-remove-jsx-empty-expression/README.md)
- [`@svgr2/babel-plugin-replace-jsx-attribute-value`](../babel-plugin-replace-jsx-attribute-value/README.md)
- [`@svgr2/babel-plugin-svg-dynamic-title`](../babel-plugin-svg-dynamic-title/README.md)
- [`@svgr2/babel-plugin-svg-em-dimensions`](../babel-plugin-svg-em-dimensions/README.md)
- [`@svgr2/babel-plugin-transform-react-native-svg`](../babel-plugin-transform-react-native-svg/README.md)
- [`@svgr2/babel-plugin-transform-svg-component`](../babel-plugin-transform-svg-component/README.md)

If you want to create your own, reading [Babel Handbook](https://github.com/jamiebuilds/babel-handbook/blob/master/translations/en/plugin-handbook.md) is a good start!

## License

MIT
