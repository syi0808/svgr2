import { cosmiconfigSync } from 'cosmiconfig';
import type { Config, State } from '@svgr2/core';

const explorer = cosmiconfigSync('svgo', {
  searchPlaces: [
    'package.json',
    '.svgorc',
    '.svgorc.js',
    '.svgorc.json',
    '.svgorc.yaml',
    '.svgorc.yml',
    'svgo.config.js',
    'svgo.config.cjs',
    '.svgo.yml',
  ],
  transform: (result) => result && result.config,
  cache: true,
});

const getSvgoConfigFromSvgrConfig = (config: Config): any => {
  const overrides: Record<string, any> = {};

  if (config.native) {
    overrides.inlineStyles = {
      onlyMatchedOnce: false,
    };
  }

  const plugins: any[] = [
    {
      name: 'preset-default',
      params: {
        overrides,
      },
    },
  ];

  if (!config.icon && config.dimensions !== false) {
    plugins.push('removeViewBox');
  }

  plugins.push('prefixIds');

  return { plugins };
};

export const getSvgoConfig = (config: Config, state: State): any => {
  const cwd = state.filePath || process.cwd();
  if (config.svgoConfig) return config.svgoConfig;
  if (config.runtimeConfig) {
    const userConfig = explorer.search(cwd);
    if (userConfig) return userConfig;
  }
  return getSvgoConfigFromSvgrConfig(config);
};
