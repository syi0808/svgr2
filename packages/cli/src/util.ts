import fs from 'node:fs/promises';
import c from 'tinyrainbow';
import { transform, Config, State } from '@svgr2/core';
import svgo from '@svgr2/plugin-svgo';
import jsx from '@svgr2/plugin-jsx-oxc';
import { camelCase, snakeCase, kebabCase, pascalCase } from 'change-case';

const { red } = c;

export function transformFilename(
  filename: string,
  filenameCase: string,
): string {
  switch (filenameCase) {
    case 'kebab':
      return kebabCase(filename);
    case 'camel':
      return camelCase(filename);
    case 'pascal':
      return pascalCase(filename);
    case 'snake':
      return snakeCase(filename);
    default:
      throw new Error(`Unknown --filename-case ${filenameCase}`);
  }
}

export function convert(
  code: string,
  config: Config,
  state: Partial<State>,
): string {
  return transform.sync(code, config, {
    ...state,
    caller: {
      name: '@svgr2/cli',
      defaultPlugins: [svgo, jsx],
    },
  });
}

export async function convertFile(
  filePath: string,
  config: Config = {},
): Promise<string> {
  const code = await fs.readFile(filePath, 'utf-8');
  return convert(code, config, { filePath });
}

export function exitError(error: string): never {
  console.error(red(error));
  process.exit(1);
}

export function politeWrite(data: string, silent?: boolean): void {
  if (!silent) {
    process.stdout.write(data);
  }
}

export function formatExportName(name: string): string {
  if (/[-]/g.test(name) && /^\d/.test(name)) {
    return `Svg${pascalCase(name)}`;
  }

  if (/^\d/.test(name)) {
    return `Svg${name}`;
  }

  return pascalCase(name);
}
