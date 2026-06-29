import { optimize } from 'svgo';
import { getSvgoConfig } from './config';
import type { Plugin } from '@svgr2/core';

const svgoPlugin: Plugin<ReturnType<typeof getSvgoConfig>> = (code, config, state, options) => {
  const svgoConfig = getSvgoConfig(config, state);
  const result = optimize(code, { ...svgoConfig, ...options, path: state.filePath });

  // @ts-expect-error
  if (result.modernError) {
    // @ts-expect-error
    throw result.modernError;
  }

  return result.data;
};

export default svgoPlugin;
