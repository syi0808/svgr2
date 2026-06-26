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

const defaultPrefixIds = {
  delim: '__',
  prefix: { type: 'Default' } as const,
  prefixIds: true,
  prefixClassNames: true,
};

export function getOxvgConfigFromOxvgConfig(config: Config): Jobs {
  const jobs: Jobs = {};

  if (config.native) {
    jobs.inlineStyles = {
      ...defaultInlineStyles,
      onlyMatchedOnce: false,
    } as const;
  }

  if (!config.icon && config.dimensions !== false) {
    jobs.removeViewBox = enabledBoolJob;
  }

  jobs.prefixIds = defaultPrefixIds;

  return extend({ type: 'Default' }, jobs);
}

export const getOxvgConfig = (config: Config, _state: State): Jobs => {
  return getOxvgConfigFromOxvgConfig(config);
};
