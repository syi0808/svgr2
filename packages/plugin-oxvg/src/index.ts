import { optimise } from '@oxvg/napi';
import { getOxvgConfig } from './config';
import type { Plugin } from '@svgr2/core';

const oxvgPlugin: Plugin<ReturnType<typeof getOxvgConfig>> = (code, config, _state, options) => {
  const result = optimise(code, { ...getOxvgConfig(config), ...options });

  if (!result) {
    throw new Error("oxvg can not optimize svg");
  }

  return result;
};

export default oxvgPlugin;
