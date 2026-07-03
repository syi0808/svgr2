import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm', 'cjs'],
  dts: {
    enabled: true,
    sourcemap: false,
  },
  clean: true,
});
