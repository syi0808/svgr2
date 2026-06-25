import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm', 'cjs'],
  dts: {
    sourcemap: false,
  },
  deps: {
    dts: {
      neverBundle: ['webpack'],
    },
  },
  clean: true,
});
