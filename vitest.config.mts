import { configDefaults, defineConfig, mergeConfig } from 'vitest/config';

export const globalConfig = defineConfig({
  test: {
    exclude: [...configDefaults.exclude, 'archive/**'],
    globals: true,
  },
});

export default mergeConfig(globalConfig, {
  test: {
    include: ['tests/*.test.ts'],
  },
});
