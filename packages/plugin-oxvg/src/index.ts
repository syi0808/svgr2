import { optimise } from '@oxvg/napi';
import { getOxvgConfig } from './config';
import type { Plugin } from '@svgr2/core';

const oxvgPlugin: Plugin = (code, config, state) => {
  const result = optimise(code, getOxvgConfig(config, state));

  if (!result) {
    throw new Error("oxvg can not optimize svg");
  }

  return result;
};

export default oxvgPlugin;
