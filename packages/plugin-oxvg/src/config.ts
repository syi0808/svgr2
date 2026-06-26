import type { Jobs } from '@oxvg/napi';
import { extend } from '@oxvg/napi';
import type { Config, State } from '@svgr2/core';

export const getOxvgConfigFromOxvgConfig = (config: Config): Jobs => {
  const jobs: Jobs = {};

  if (config.native) {
    jobs.inlineStyles = {
      onlyMatchedOnce: false,
    };
  }

  if (!config.icon && config.dimensions !== false) {
    jobs.removeViewBox = true;
  }

  // 기존 SVGR 기본값처럼 prefixIds를 항상 켜고 싶다면
  jobs.prefixIds = {};

  return extend('default', jobs);
};

export const getOxvgConfig = (config: Config, _state: State): Jobs => {
  return getOxvgConfigFromOxvgConfig(config);
};
