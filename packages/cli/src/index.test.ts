import { promises as fs } from 'fs';
import * as path from 'path';
import { exec as execCb } from 'child_process';
import { promisify } from 'util';

const exec = promisify(execCb);

const packageRoot = path.join(__dirname, '..');
const packagePath = (...paths: string[]) => path.join(packageRoot, ...paths);
const svgr = packagePath('bin/svgr');

const sortCliResult = (result: string) => {
  return result
    .split(/\n/)
    .sort((a, b) => a.localeCompare(b))
    .map((x) => x.toLowerCase())
    .join('\n');
};

describe('cli', () => {
  const cli = async (args: string) => {
    const { stdout } = await exec(`${svgr} ${args}`, { cwd: packageRoot });
    return stdout;
  };

  it('should work with a simple file', async () => {
    const result = await cli('tests/__fixtures__/simple/file.svg');
    expect(result).toMatchSnapshot();
  });

  it('should not work with a directory without --out-dir option', async () => {
    expect.assertions(1);
    try {
      await cli('tests/__fixtures__/nesting');
    } catch (error: any) {
      expect(error.message).toMatch(
        'Directory are not supported without `--out-dir` option instead',
      );
    }
  });

  it('should not work with several files without destination', async () => {
    expect.assertions(1);
    try {
      await cli(
        'tests/__fixtures__/simple/file.svg tests/__fixtures__/nesting/one.svg',
      );
    } catch (error: any) {
      expect(error.message).toMatch(
        'Please specify only one filename or use `--out-dir` option',
      );
    }
  });

  it('should work with stdin', async () => {
    const result = await cli('< tests/__fixtures__/simple/file.svg');
    expect(result).toMatchSnapshot();
  });

  it('should support stdin filepath', async () => {
    const result = await cli(
      '--stdin-filepath tests/__fixtures__/simple/file.svg < tests/__fixtures__/simple/file.svg',
    );
    expect(result).toMatchSnapshot();
  });

  it('should transform a whole directory and output relative destination paths', async () => {
    const result = await cli(
      '--out-dir __fixtures_build__/whole tests/__fixtures__',
    );
    expect(sortCliResult(result)).toMatchSnapshot();
  });

  it('should transform a whole directory with --typescript', async () => {
    const result = await cli(
      '--typescript --out-dir __fixtures_build__/whole tests/__fixtures__',
    );
    expect(sortCliResult(result)).toMatchSnapshot();
  });

  it('should suppress output when transforming a directory with a --silent option', async () => {
    const result = await cli(
      '--silent --out-dir __fixtures_build__/whole tests/__fixtures__',
    );
    expect(sortCliResult(result)).toMatchSnapshot();
  });

  it.each([
    ['--no-dimensions'],
    ['--jsx-runtime classic-preact'],
    ['--jsx-runtime automatic'],
    ['--expand-props none'],
    ['--expand-props start'],
    ['--icon'],
    ['--icon 24'],
    ['--icon 2em'],
    ['--native'],
    ['--native --icon'],
    ['--native --expand-props none'],
    ['--native --ref'],
    ['--ref'],
    ['--replace-attr-values "#063855=currentColor"'],
    [`--svg-props "hidden={true},id=hello"`],
    ['--title-prop'],
    ['--desc-prop'],
    ['--typescript'],
    ['--typescript --ref'],
    ['--typescript --ref --title-prop'],
    ['--typescript --ref --desc-prop'],
  ])(
    'should support various args',
    async (args) => {
      const result = await cli(`${args} -- tests/__fixtures__/simple/file.svg`);
      expect(result).toMatchSnapshot(args);
    },
    10000,
  );

  it.each([
    [0, ''],
    [1, '--filename-case=camel'],
    [2, '--filename-case=pascal'],
    [3, '--filename-case=kebab'],
    [4, '--filename-case=snake'],
  ])(
    'should support different filename cases with directory output',
    async (index, args) => {
      const inDir = 'tests/__fixtures__/cased';
      const outDir = `__fixtures_build__/filename-case-${index}`;
      await fs.rm(packagePath(outDir), { recursive: true, force: true });
      await cli(`${args} ${inDir} --out-dir=${outDir}`);
      expect(await fs.readdir(packagePath(outDir))).toMatchSnapshot(args);
    },
    10000,
  );

  it('should support custom file extension', async () => {
    const inDir = 'tests/__fixtures__/simple';
    const outDir = '__fixtures_build__/ext';
    await fs.rm(packagePath(outDir), { recursive: true, force: true });
    await cli(`--ext=ts ${inDir} --out-dir=${outDir}`);
    expect(await fs.readdir(packagePath(outDir))).toMatchSnapshot();
  });

  it('should support "--ignore-existing"', async () => {
    const inDir = 'tests/__fixtures__/simple';
    const outDir = 'tests/__fixtures__/simple-existing';
    await cli(`${inDir} --out-dir=${outDir} --ignore-existing`);
    const content = await fs.readFile(packagePath(outDir, 'File.js'), 'utf-8');
    expect(content).toBe('// nothing\n');
  });

  it('should not override config with cli defaults', async () => {
    const result = await cli(
      'tests/__fixtures__/simple/file.svg --config-file=tests/__fixtures__/overrides.config.cjs',
    );
    expect(result).toMatchSnapshot();
  });

  it('should add Svg prefix to index.js exports staring with number', async () => {
    const inDir = 'tests/__fixtures__/numeric';
    const outDir = `__fixtures_build__/prefix-exports`;
    await fs.rm(packagePath(outDir), { recursive: true, force: true });
    await cli(`${inDir} --out-dir=${outDir}`);
    const content = await fs.readFile(packagePath(outDir, 'index.js'), 'utf-8');
    expect(content).toMatchSnapshot();
  });

  it('should support custom index.js with directory output', async () => {
    const inDir = 'tests/__fixtures__/simple';
    const outDir = `__fixtures_build__/custom-index`;
    await fs.rm(packagePath(outDir), { recursive: true, force: true });
    await cli(
      `${inDir} --out-dir=${outDir} --config-file=tests/__fixtures__/custom-index.config.cjs`,
    );
    const content = await fs.readFile(packagePath(outDir, 'index.js'), 'utf-8');
    expect(content).toMatchSnapshot();
  });

  it('using typescript option, it creates index with `.ts` extension', async () => {
    const inDir = 'tests/__fixtures__/simple';
    const outDir = `__fixtures_build__/ts-index`;
    await fs.rm(packagePath(outDir), { recursive: true, force: true });
    await cli(`${inDir} --out-dir=${outDir} --typescript`);
    const content = await fs.readFile(packagePath(outDir, 'index.ts'), 'utf-8');
    expect(content).toMatchSnapshot();
  });

  it('should support --index-template in cli', async () => {
    const inDir = 'tests/__fixtures__/simple';
    const outDir = `__fixtures_build__/custom-index-arg`;
    await fs.rm(packagePath(outDir), { recursive: true, force: true });
    await cli(
      `${inDir} --out-dir=${outDir} --index-template=tests/__fixtures__/custom-index-template.cjs`,
    );
    const content = await fs.readFile(packagePath(outDir, 'index.js'), 'utf-8');
    expect(content).toMatchSnapshot();
  });

  it('should support --no-index', async () => {
    const inDir = 'tests/__fixtures__/simple';
    const outDir = `__fixtures_build__/no-index-case`;
    await fs.rm(packagePath(outDir), { recursive: true, force: true });
    await cli(`--no-index ${inDir} --out-dir=${outDir}`);
    expect(await fs.readdir(packagePath(outDir))).toMatchSnapshot();
  });
});
