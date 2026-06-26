import { optimise } from '@oxvg/napi';
import { optimize } from 'svgo';
import path from 'node:path';
import { readFileSync } from 'node:fs';
import { bench, describe } from 'vitest';

const fixtureDir = path.join(process.cwd(), 'fixtures/svg');

const hugePathSvg = readFileSync(path.join(fixtureDir, "huge-path.svg"), 'utf8');;

describe("huge path", () => {
  bench("oxvg", () => {
    optimise(hugePathSvg);
  })

  bench("svgo", () => {
    optimize(hugePathSvg);
  })
})