import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const fixtureDir = join(process.cwd(), 'fixtures/svg');

export interface Fixture {
  name: string;
  file: string;
  componentName?: string;
}

export interface LoadedFixture {
  name: string;
  source: string;
  componentName: string;
  filePath: string;
}

const toComponentName = (file: string) =>
  file
    .replace(/\.svg$/, '')
    .split(/[-_]/g)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join('');

export const loadFixture = ({
  name,
  file,
  componentName = toComponentName(file),
}: Fixture): LoadedFixture => {
  const filePath = join(fixtureDir, file);

  return {
    name,
    source: readFileSync(filePath, 'utf8'),
    componentName,
    filePath,
  };
};
