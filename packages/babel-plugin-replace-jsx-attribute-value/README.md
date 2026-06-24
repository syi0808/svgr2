# @svgr2/babel-plugin-replace-jsx-attribute-value

## Install

```
npm install --save-dev @svgr2/babel-plugin-replace-jsx-attribute-value
```

## Usage

**.babelrc**

```json
{
  "plugins": [
    [
      "@svgr2/babel-plugin-replace-jsx-attribute-value",
      {
        "values": [
          { "value": "#000", "newValue": "#fff" },
          { "value": "blue", "newValue": "props.color", "literal": true }
        ]
      }
    ]
  ]
}
```

## License

MIT
