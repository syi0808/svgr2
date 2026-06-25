import { transform, type TransformOptions } from '@svgr2/oxc-transform-svg-jsx-napi';
import type { Plugin, Config } from '@svgr2/core';

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
      return {
        jsxRuntime: 'classic',
        importSource: 'react',
        jsxRuntimeImport: { namespace: 'React', source: 'react' },
      };
    case 'classic-preact':
      return {
        jsxRuntime: 'classic',
        importSource: 'preact/compat',
        jsxRuntimeImport: { specifiers: ['h'], source: 'preact' },
      };
    case 'automatic':
      return { jsxRuntime: 'automatic' };
    default:
      throw new Error(`Unsupported "jsxRuntime" "${config.jsxRuntime}"`);
  }
};

const jsxPlugin: Plugin = (code, config, state) => {
  const result = transform(code, {
    componentName: state.componentName,
    previousExport: state.caller?.previousExport ?? undefined,
    ref: config.ref,
    titleProp: config.titleProp,
    descProp: config.descProp,
    expandProps: config.expandProps,
    dimensions: config.dimensions,
    icon: config.icon,
    native: config.native,
    svgProps: config.svgProps,
    replaceAttrValues: config.replaceAttrValues,
    typescript: config.typescript,
    memo: config.memo,
    exportType: config.exportType,
    namedExport: config.namedExport,
    ...getJsxRuntimeOptions(config),
  });

  if (!result) {
    throw new Error(`Unable to generate SVG file`);
  }

  return result;
};

export default jsxPlugin;
