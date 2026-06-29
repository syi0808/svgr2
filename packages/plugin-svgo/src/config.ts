import { cosmiconfigSync } from 'cosmiconfig';
import type { Config, State } from '@svgr2/core';
import type { Config as SvgoConfig } from 'svgo';

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

const getSvgoConfigFromSvgrConfig = (config: Config): SvgoConfig => {
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

export const getSvgoConfig = (config: Config, state: State): SvgoConfig => {
  const cwd = state.filePath || process.cwd();

  if (config.runtimeConfig) {
    const userConfig = explorer.search(cwd);
    if (userConfig) return userConfig as SvgoConfig;
  }

  return getSvgoConfigFromSvgrConfig(config);
};
