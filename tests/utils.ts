import path from 'node:path';

export function fixture(fixturePath: string) {
  return path.resolve(
    import.meta.dirname,
    '../packages/cli/tests/__fixtures__',
    fixturePath,
  );
}
