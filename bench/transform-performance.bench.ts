import {
  transform,
  type Config as SvgrConfig,
  type ConfigPlugin as SvgrPlugin,
} from '@svgr/core';
import svgo from '@svgr/plugin-svgo';
import {
  createTransformerSync,
  transform as svgr2Transform,
  type Config as Svgr2Config,
} from '@svgr2/core';
import oxvgPlugin from '@svgr2/plugin-oxvg';
import { bench, describe } from 'vitest';
import { loadFixture, type Fixture, type LoadedFixture } from './fixtures.js';

type Svgr2BenchmarkConfig = Svgr2Config & { svgo?: boolean };

interface TransformCase {
  name: string;
  fixtures: readonly Fixture[];
  benchmarkSuffix?: string;
  svgrConfig: SvgrConfig;
  svgr2Config: Svgr2BenchmarkConfig;
}

const fixtures = [
  { name: 'tiny', file: 'tiny.svg' },
  { name: 'many attributes', file: 'many-attributes.svg' },
  { name: 'huge path', file: 'huge-path.svg' },
  { name: 'deep nesting', file: 'deep-nesting.svg' },
  { name: 'style heavy', file: 'style-heavy.svg' },
  { name: 'entities text', file: 'entities-text.svg' },
  { name: 'comments cdata', file: 'comments-cdata.svg' },
] satisfies readonly Fixture[];

const defaultSvgrConfig = {
  plugins: [svgo as unknown as SvgrPlugin, '@svgr/plugin-jsx'],
  svgo: true,
  icon: true,
} satisfies SvgrConfig;

const defaultSvgr2Config = {
  plugins: [oxvgPlugin, '@svgr2/plugin-jsx-oxc'],
  icon: true,
} satisfies Svgr2BenchmarkConfig;

const replaceAttrValues = {
  '#000': 'currentColor',
  '#000000': 'currentColor',
  '#111': '{props.color}',
  '#111111': '{props.color}',
  red: '{props.color}',
  blue: 'currentColor',
};

const cases: readonly TransformCase[] = [
  {
    name: 'default',
    fixtures,
    svgrConfig: defaultSvgrConfig,
    svgr2Config: defaultSvgr2Config,
  },
  {
    name: 'replaceAttrValues',
    fixtures: [
      {
        name: 'replaceAttrValues',
        file: 'replace-heavy.svg',
        componentName: 'ReplaceHeavy',
      },
    ],
    svgrConfig: { ...defaultSvgrConfig, replaceAttrValues },
    svgr2Config: { ...defaultSvgr2Config, replaceAttrValues },
  },
  {
    name: 'native',
    fixtures: [
      { name: 'native', file: 'native.svg', componentName: 'NativeIcon' },
    ],
    svgrConfig: { ...defaultSvgrConfig, native: true },
    svgr2Config: { ...defaultSvgr2Config, native: true },
  },
  {
    name: 'title-desc',
    fixtures: [
      {
        name: 'title-desc',
        file: 'title-desc.svg',
        componentName: 'TitleDescIcon',
      },
    ],
    svgrConfig: { ...defaultSvgrConfig, titleProp: true, descProp: true },
    svgr2Config: { ...defaultSvgr2Config, titleProp: true, descProp: true },
  },
  {
    name: 'jsx-only',
    fixtures,
    benchmarkSuffix: ' jsx-only',
    svgrConfig: {
      plugins: ['@svgr/plugin-jsx'],
      svgo: false,
      icon: true,
    },
    svgr2Config: {
      plugins: ['@svgr2/plugin-jsx-oxc'],
      svgo: false,
      icon: true,
    },
  },
];

const registerBenchmarks = (
  fixture: LoadedFixture,
  benchmarkCase: TransformCase,
) => {
  const state = {
    componentName: fixture.componentName,
    filePath: fixture.filePath,
  };
  const transformer = createTransformerSync(benchmarkCase.svgr2Config, state);
  const suffix = benchmarkCase.benchmarkSuffix ?? '';

  bench(`svgr${suffix}`, async () => {
    await transform(fixture.source, benchmarkCase.svgrConfig, state);
  });

  bench(`svgr2${suffix}`, async () => {
    await svgr2Transform(fixture.source, benchmarkCase.svgr2Config, state);
  });

  bench(`svgr2${suffix} with transformer`, () => {
    transformer.transform(fixture.source);
  });
};

for (const benchmarkCase of cases) {
  describe(`transform-jsx/fixtures/${benchmarkCase.name}`, () => {
    const loadedFixtures = benchmarkCase.fixtures.map(loadFixture);

    for (const fixture of loadedFixtures) {
      if (loadedFixtures.length === 1) {
        registerBenchmarks(fixture, benchmarkCase);
      } else {
        describe(fixture.name, () => {
          registerBenchmarks(fixture, benchmarkCase);
        });
      }
    }
  });
}
