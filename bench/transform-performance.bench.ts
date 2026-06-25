import { describe, bench } from 'vitest';

// Original Svgr
import { transform } from '@svgr/core';

// New Svgr2
import {
  transform as svgr2Transform,
  createTransformerSync,
} from '@svgr2/core';

const svgCode = `
<svg xmlns="http://www.w3.org/2000/svg"
  xmlns:xlink="http://www.w3.org/1999/xlink">
  <rect x="10" y="10" height="100" width="100"
    style="stroke:#ff0000; fill: #0000ff"/>
</svg>
`;

const svgr2Transformer = createTransformerSync(
  {
    plugins: ['@svgr2/plugin-svgo', '@svgr2/plugin-jsx-oxc'],
    icon: true,
  },
  { componentName: 'MyComponent' },
);

describe('transform-jsx', () => {
  bench('svgr', async () => {
    await transform(
      svgCode,
      {
        plugins: ['@svgr/plugin-svgo', '@svgr/plugin-jsx'],
        svgo: true,
        icon: true,
      },
      { componentName: 'MyComponent' },
    );
  });

  bench('svgr2', async () => {
    await svgr2Transform(
      svgCode,
      {
        plugins: ['@svgr2/plugin-svgo', '@svgr2/plugin-jsx-oxc'],
        icon: true,
      },
      { componentName: 'MyComponent' },
    );
  });

  bench('svgr2 with transformer', () => {
    svgr2Transformer.transform(svgCode);
  });
});
