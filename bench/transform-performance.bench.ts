import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, bench } from 'vitest';

// Original Svgr
import { transform } from '@svgr/core';
import svgo from '@svgr/plugin-svgo';

// New Svgr2
import {
  transform as svgr2Transform,
  createTransformerSync,
} from '@svgr2/core';
import oxvgPlugin from '@svgr2/plugin-oxvg';

const fixtureDir = join(process.cwd(), 'fixtures/svg');

const readFixture = (name: string) =>
  readFileSync(join(fixtureDir, name), 'utf8');

const toComponentName = (name: string) =>
  name
    .replace(/\.svg$/, '')
    .split(/[-_]/g)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join('');

const svgCode = `
<svg xmlns="http://www.w3.org/2000/svg"
  xmlns:xlink="http://www.w3.org/1999/xlink">
  <rect x="10" y="10" height="100" width="100"
    style="stroke:#ff0000; fill: #0000ff"/>
</svg>
`;

const defaultSvgrPlugins = [svgo, '@svgr/plugin-jsx'];
const defaultSvgr2Plugins = [oxvgPlugin, '@svgr2/plugin-jsx-oxc'];

const defaultSvgrConfig = {
  plugins: defaultSvgrPlugins,
  svgo: true,
  icon: true,
};

const defaultSvgr2Config = {
  plugins: defaultSvgr2Plugins,
  icon: true,
};

const svgr2Transformer = createTransformerSync(defaultSvgr2Config, {
  componentName: 'MyComponent',
});

describe('transform-jsx/basic', () => {
  bench('svgr', async () => {
    await transform(svgCode, defaultSvgrConfig, {
      componentName: 'MyComponent',
    });
  });

  bench('svgr2', async () => {
    await svgr2Transform(svgCode, defaultSvgr2Config, {
      componentName: 'MyComponent',
    });
  });

  bench('svgr2 with transformer', () => {
    svgr2Transformer.transform(svgCode);
  });
});

const cases = [
  {
    name: 'tiny',
    file: 'tiny.svg',
  },
  {
    name: 'many attributes',
    file: 'many-attributes.svg',
  },
  {
    name: 'huge path',
    file: 'huge-path.svg',
  },
  {
    name: 'deep nesting',
    file: 'deep-nesting.svg',
  },
  {
    name: 'style heavy',
    file: 'style-heavy.svg',
  },
  {
    name: 'entities text',
    file: 'entities-text.svg',
  },
  {
    name: 'comments cdata',
    file: 'comments-cdata.svg',
  },
] as const;

describe('transform-jsx/fixtures/default', () => {
  for (const item of cases) {
    const source = readFixture(item.file);
    const componentName = toComponentName(item.file);

    const transformer = createTransformerSync(defaultSvgr2Config, {
      componentName,
      filePath: join(fixtureDir, item.file),
    });

    describe(item.name, () => {
      bench('svgr', async () => {
        await transform(source, defaultSvgrConfig, {
          componentName,
          filePath: join(fixtureDir, item.file),
        });
      });

      bench('svgr2', async () => {
        await svgr2Transform(source, defaultSvgr2Config, {
          componentName,
          filePath: join(fixtureDir, item.file),
        });
      });

      bench('svgr2 with transformer', () => {
        transformer.transform(source);
      });
    });
  }
});

describe('transform-jsx/fixtures/replaceAttrValues', () => {
  const file = 'replace-heavy.svg';
  const source = readFixture(file);
  const componentName = 'ReplaceHeavy';
  const filePath = join(fixtureDir, file);

  const svgrConfig = {
    ...defaultSvgrConfig,
    replaceAttrValues: {
      '#000': 'currentColor',
      '#000000': 'currentColor',
      '#111': '{props.color}',
      '#111111': '{props.color}',
      red: '{props.color}',
      blue: 'currentColor',
    },
  };

  const svgr2Config = {
    ...defaultSvgr2Config,
    replaceAttrValues: {
      '#000': 'currentColor',
      '#000000': 'currentColor',
      '#111': '{props.color}',
      '#111111': '{props.color}',
      red: '{props.color}',
      blue: 'currentColor',
    },
  };

  const transformer = createTransformerSync(svgr2Config, {
    componentName,
    filePath,
  });

  bench('svgr', async () => {
    await transform(source, svgrConfig, {
      componentName,
      filePath,
    });
  });

  bench('svgr2', async () => {
    await svgr2Transform(source, svgr2Config, {
      componentName,
      filePath,
    });
  });

  bench('svgr2 with transformer', () => {
    transformer.transform(source);
  });
});

describe('transform-jsx/fixtures/native', () => {
  const file = 'native.svg';
  const source = readFixture(file);
  const componentName = 'NativeIcon';
  const filePath = join(fixtureDir, file);

  const svgrConfig = {
    ...defaultSvgrConfig,
    native: true,
  };

  const svgr2Config = {
    ...defaultSvgr2Config,
    native: true,
  };

  const transformer = createTransformerSync(svgr2Config, {
    componentName,
    filePath,
  });

  bench('svgr', async () => {
    await transform(source, svgrConfig, {
      componentName,
      filePath,
    });
  });

  bench('svgr2', async () => {
    await svgr2Transform(source, svgr2Config, {
      componentName,
      filePath,
    });
  });

  bench('svgr2 with transformer', () => {
    transformer.transform(source);
  });
});

describe('transform-jsx/fixtures/title-desc', () => {
  const file = 'title-desc.svg';
  const source = readFixture(file);
  const componentName = 'TitleDescIcon';
  const filePath = join(fixtureDir, file);

  const svgrConfig = {
    ...defaultSvgrConfig,
    titleProp: true,
    descProp: true,
  };

  const svgr2Config = {
    ...defaultSvgr2Config,
    titleProp: true,
    descProp: true,
  };

  const transformer = createTransformerSync(svgr2Config, {
    componentName,
    filePath,
  });

  bench('svgr', async () => {
    await transform(source, svgrConfig, {
      componentName,
      filePath,
    });
  });

  bench('svgr2', async () => {
    await svgr2Transform(source, svgr2Config, {
      componentName,
      filePath,
    });
  });

  bench('svgr2 with transformer', () => {
    transformer.transform(source);
  });
});

describe('transform-jsx/fixtures/jsx-only', () => {
  for (const item of cases) {
    const source = readFixture(item.file);
    const componentName = toComponentName(item.file);
    const filePath = join(fixtureDir, item.file);

    const svgrConfig = {
      plugins: ['@svgr/plugin-jsx'],
      svgo: false,
      icon: true,
    };

    const svgr2Config = {
      plugins: ['@svgr2/plugin-jsx-oxc'],
      svgo: false,
      icon: true,
    };

    const transformer = createTransformerSync(svgr2Config, {
      componentName,
      filePath,
    });

    describe(item.name, () => {
      bench('svgr jsx-only', async () => {
        await transform(source, svgrConfig, {
          componentName,
          filePath,
        });
      });

      bench('svgr2 jsx-only', async () => {
        await svgr2Transform(source, svgr2Config, {
          componentName,
          filePath,
        });
      });

      bench('svgr2 jsx-only with transformer', () => {
        transformer.transform(source);
      });
    });
  }
});