import { transform as wasmTransform } from './pkg/oxc_transform_svg_jsx_wasm.js';

function toOptionsJson(options) {
  if (options == null) return undefined;
  if (typeof options === 'string') return options;
  return JSON.stringify(options);
}

export function transform(source, options) {
  return wasmTransform(source, toOptionsJson(options));
}
