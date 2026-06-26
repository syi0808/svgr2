import type { Jobs } from '@oxvg/napi';
import { extend } from '@oxvg/napi';
import type { Config, State } from '@svgr2/core';

const enabledBoolJob = { field0: true };

const defaultInlineStyles = {
  onlyMatchedOnce: true,
  removeMatchedSelectors: true,
  useMqs: ['', 'screen'],
  usePseudos: [''],
};

const nativeInlineStyles = {
  ...defaultInlineStyles,
  onlyMatchedOnce: false,
} as const

const defaultPrefixIds = {
  delim: '__',
  prefix: { type: 'Default' } as const,
  prefixIds: true,
  prefixClassNames: true,
};

const configCache = new WeakMap();

export const getOxvgConfig = (config: Config): Jobs => {
  if (configCache.get(config)) return configCache.get(config);

  const jobs: Jobs = {};

  if (config.native) {
    jobs.inlineStyles = nativeInlineStyles;
  }

  if (!config.icon && config.dimensions !== false) {
    jobs.removeViewBox = enabledBoolJob;
  }

  jobs.prefixIds = defaultPrefixIds;

  const result = extend({ type: 'Default' }, jobs);

  configCache.set(config, result);

  return result;
};
