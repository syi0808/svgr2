import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    include: ['*.bench.ts'],
    fileParallelism: false,
    benchmark: {
      includeSamples: false,
    },
  },
})