import { extend, type Jobs } from '@svgr2/combined-napi';
import type { Config } from '@svgr2/core';

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
};

const defaultPrefixIds = {
  delim: '__',
  prefix: { type: 'Default' } as const,
  prefixIds: true,
  prefixClassNames: true,
};

const configCache = new WeakMap<Config, Jobs>();

export const getOxvgConfig = (config: Config): Jobs => {
  const cached = configCache.get(config);
  if (cached) return cached;

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
