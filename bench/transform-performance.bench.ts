import { describe, bench } from 'vitest';
import { transform } from '@svgr/core';

const svgCode = `
<svg xmlns="http://www.w3.org/2000/svg"
  xmlns:xlink="http://www.w3.org/1999/xlink">
  <rect x="10" y="10" height="100" width="100"
    style="stroke:#ff0000; fill: #0000ff"/>
</svg>
`;

describe('transform-jsx', () => {
  bench("svgr", async () => {
    await transform(
      svgCode,
      {
        plugins: ['@svgr/plugin-svgo', '@svgr/plugin-jsx'],
        svgo: true,
        icon: true,
      },
      { componentName: 'MyComponent' },
    )
  });
});