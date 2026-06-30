import { transform as wasmTransform } from './pkg/combined_wasm.js';

function toOptionsJson(options) {
  if (options == null) return undefined;
  if (typeof options === 'string') return options;
  return JSON.stringify(options);
}

export function transform(source, transformOptions, optimiseOptions) {
  return wasmTransform(
    source,
    toOptionsJson(transformOptions),
    toOptionsJson(optimiseOptions),
  );
}
