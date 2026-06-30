import {
  transform,
  type Jobs,
  type TransformOptions,
} from '@svgr2/combined-napi';
import type { Config, Plugin } from '@svgr2/core';
import { getOxvgConfig } from './config.js';

const CLASSIC_JSX_RUNTIME_OPTIONS = {
  jsxRuntime: 'classic',
  importSource: 'react',
  jsxRuntimeImport: { namespace: 'React', source: 'react' },
} as const;

const CLASSIC_PREACT_JSX_RUNTIME_OPTIONS = {
  jsxRuntime: 'classic',
  importSource: 'preact/compat',
  jsxRuntimeImport: { specifiers: ['h'] as string[], source: 'preact' },
} as const;

const AUTOMATIC_JSX_RUNTIME_OPTIONS = { jsxRuntime: 'automatic' } as const;

const getJsxRuntimeOptions = (config: Config): Partial<TransformOptions> => {
  if (config.jsxRuntimeImport) {
    return {
      importSource: config.jsxRuntimeImport.source,
      jsxRuntimeImport: config.jsxRuntimeImport,
    };
  }
  switch (config.jsxRuntime) {
    case null:
    case undefined:
    case 'classic':
      return CLASSIC_JSX_RUNTIME_OPTIONS;
    case 'classic-preact':
      return CLASSIC_PREACT_JSX_RUNTIME_OPTIONS;
    case 'automatic':
      return AUTOMATIC_JSX_RUNTIME_OPTIONS;
    default:
      throw new Error(`Unsupported "jsxRuntime" "${config.jsxRuntime}"`);
  }
};

const combinedPlugin: Plugin<Jobs> = (code, config, state, options) => {
  const result = transform(
    code,
    {
      ...config,
      ...(state.componentName && { componentName: state.componentName }),
      ...(state.caller?.previousExport && {
        previousExport: state.caller.previousExport,
      }),
      ...getJsxRuntimeOptions(config),
    },
    { ...getOxvgConfig(config), ...options },
  );

  if (!result) {
    throw new Error('Unable to optimise and transform SVG');
  }

  return result;
};

export default combinedPlugin;
export { combinedPlugin };
