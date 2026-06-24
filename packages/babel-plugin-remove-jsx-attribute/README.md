# @svgr2/babel-plugin-remove-jsx-attribute

## Install

```
npm install --save-dev @svgr2/babel-plugin-remove-jsx-attribute
```

## Usage

**.babelrc**

```json
{
  "plugins": [
    [
      "@svgr2/babel-plugin-remove-jsx-attribute",
      {
        "elements": ["svg"],
        "attributes": ["width", "height"]
      }
    ]
  ]
}
```

## License

MIT
