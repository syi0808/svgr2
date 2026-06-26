import { optimise } from '@oxvg/napi';
import { getOxvgConfig } from './config';
import type { Plugin } from '@svgr2/core';

const oxvgPlugin: Plugin = (code, config, _state) => {
  const result = optimise(code, getOxvgConfig(config));

  if (!result) {
    throw new Error("oxvg can not optimize svg");
  }

  return result;
};

export default oxvgPlugin;
