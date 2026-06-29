import { optimise } from '@oxvg/napi';
import { optimize } from 'svgo';
import { bench, describe } from 'vitest';
import { loadFixture, type Fixture } from './fixtures.js';

const benchmarkOptions = { time: 1500, warmupTime: 300 };

interface OptimizeCase {
  fixture: Fixture;
  implementations: ReadonlyArray<{
    name: string;
    optimize: (source: string) => unknown;
  }>;
}

const cases = [
  {
    fixture: { name: 'huge path', file: 'huge-path.svg' },
    implementations: [
      { name: 'oxvg', optimize: optimise },
      { name: 'svgo', optimize },
    ],
  },
] satisfies readonly OptimizeCase[];

for (const benchmarkCase of cases) {
  const fixture = loadFixture(benchmarkCase.fixture);

  describe(fixture.name, () => {
    for (const implementation of benchmarkCase.implementations) {
      bench(
        implementation.name,
        () => {
          implementation.optimize(fixture.source);
        },
        benchmarkOptions,
      );
    }
  });
}
