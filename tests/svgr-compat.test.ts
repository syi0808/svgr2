import generate from '@babel/generator';
import { parse } from '@babel/parser';
import {
  transform as transformWithSvgr,
  type Config as SvgrConfig,
  type State as SvgrState,
} from '@svgr/core';
import svgrJsx from '@svgr/plugin-jsx';
import svgrSvgo from '@svgr/plugin-svgo';
import {
  transform as transformWithSvgr2,
  type Config as Svgr2Config,
  type State as Svgr2State,
} from '@svgr2/core';
import svgr2Jsx from '@svgr2/plugin-jsx-oxc';
import svgr2Oxvg from '@svgr2/plugin-oxvg';
import svgr2Svgo from '@svgr2/plugin-svgo';

type SharedConfig = Omit<SvgrConfig, 'plugins' | 'svgo'> &
  Omit<Svgr2Config, 'plugins'>;
type SharedState = Partial<SvgrState & Svgr2State>;

interface ComparisonCase {
  name: string;
  config?: SharedConfig;
  source?: string;
  state?: SharedState;
}

interface OptimizerComparisonCase extends ComparisonCase {
  source: string;
  svgoParity: 'equal' | 'different';
}

const baseSvg = `<svg width="88px" height="88px" viewBox="0 0 88 88">
  <title>Dismiss</title>
  <desc>Description</desc>
  <g stroke="none" fill="none">
    <path d="M0 0" />
  </g>
</svg>`;

const canonicalizeModule = (code: string): string => {
  const ast = parse(code, {
    plugins: ['jsx', 'typescript'],
    sourceType: 'module',
  });
  const imports = ast.program.body.flatMap((statement) => {
    if (statement.type !== 'ImportDeclaration') return [];

    return statement.specifiers.map((specifier) => {
      if (specifier.type === 'ImportDefaultSpecifier') {
        return `${statement.source.value}:default:${specifier.local.name}`;
      }
      if (specifier.type === 'ImportNamespaceSpecifier') {
        return `${statement.source.value}:namespace:${specifier.local.name}`;
      }

      const imported =
        specifier.imported.type === 'Identifier'
          ? specifier.imported.name
          : specifier.imported.value;
      return `${statement.source.value}:named:${imported}:${specifier.local.name}`;
    });
  });
  ast.program.body = ast.program.body.filter(
    (statement) => statement.type !== 'ImportDeclaration',
  );

  return `${imports.sort().join('\n')}\n${generate(ast, { comments: false }).code}`;
};

const getState = (state: SharedState = {}): SharedState => ({
  componentName: 'SvgComponent',
  ...state,
});

const transformJsxOnly = async ({
  config = {},
  source = baseSvg,
  state,
}: ComparisonCase) => {
  const resolvedState = getState(state);
  const [svgr, svgr2] = await Promise.all([
    transformWithSvgr(
      source,
      {
        ...config,
        plugins: [svgrJsx],
        svgo: false,
      } as SvgrConfig,
      resolvedState,
    ),
    transformWithSvgr2(
      source,
      {
        ...config,
        plugins: [svgr2Jsx],
      } as Svgr2Config,
      resolvedState,
    ),
  ]);

  return { svgr, svgr2 };
};

const jsxOnlyCases: ComparisonCase[] = [
  { name: 'default' },
  { name: 'automatic runtime', config: { jsxRuntime: 'automatic' } },
  { name: 'preact runtime', config: { jsxRuntime: 'classic-preact' } },
  { name: 'dimensions disabled', config: { dimensions: false } },
  { name: 'props disabled', config: { expandProps: false } },
  { name: 'props at start', config: { expandProps: 'start' } },
  { name: 'default icon', config: { icon: true } },
  { name: 'numeric icon', config: { icon: 24 } },
  { name: 'string icon', config: { icon: '2em' } },
  { name: 'ref', config: { ref: true } },
  { name: 'memo', config: { memo: true } },
  { name: 'ref and memo', config: { ref: true, memo: true } },
  { name: 'title prop', config: { titleProp: true } },
  { name: 'description prop', config: { descProp: true } },
  {
    name: 'title and description props',
    config: { titleProp: true, descProp: true },
  },
  {
    name: 'SVG and replacement props',
    config: {
      replaceAttrValues: { none: '{props.color}' },
      svgProps: { role: 'img' },
    },
  },
  {
    name: 'native',
    config: { native: true },
  },
  {
    name: 'native icon and ref',
    config: { icon: true, native: true, ref: true },
  },
  {
    name: 'TypeScript props',
    config: {
      descProp: true,
      ref: true,
      titleProp: true,
      typescript: true,
    },
  },
  {
    name: 'named export with previous export',
    config: { exportType: 'named', namedExport: 'Component' },
    state: {
      caller: {
        name: 'compatibility-test',
        previousExport: 'export default "icon.svg";',
      },
    },
  },
];

describe('SVGR and Svgr2 JSX compatibility', () => {
  it.each(jsxOnlyCases)('$name', async (comparisonCase) => {
    const output = await transformJsxOnly(comparisonCase);

    expect(canonicalizeModule(output.svgr)).toBe(
      canonicalizeModule(output.svgr2),
    );
    expect(output).toMatchSnapshot();
  });
});

const optimizerCases: OptimizerComparisonCase[] = [
  {
    name: 'basic icon',
    config: { icon: true },
    source:
      '<svg viewBox="0 0 16 16"><path fill="#000" d="M8 1 L15 15 L1 15 Z" /></svg>',
    svgoParity: 'equal',
  },
  {
    name: 'entities and text',
    source:
      '<svg viewBox="0 0 24 24"><text x="2" y="12">Tom &amp; Jerry</text></svg>',
    svgoParity: 'equal',
  },
  {
    name: 'styles',
    source: `<svg viewBox="0 0 24 24">
      <style>.shape { fill: red; stroke: blue; }</style>
      <path class="shape" d="M0 0h24v24H0z" />
    </svg>`,
    svgoParity: 'equal',
  },
  {
    name: 'replacement values',
    config: {
      replaceAttrValues: {
        '#000': 'currentColor',
        red: '{props.color}',
      },
    },
    source:
      '<svg viewBox="0 0 24 24"><path fill="#000" stroke="red" d="M0 0h24v24H0z" /></svg>',
    svgoParity: 'equal',
  },
  {
    name: 'native',
    config: { native: true },
    source: '<svg viewBox="0 0 24 24"><g><path d="M0 0h24v24H0z" /></g></svg>',
    svgoParity: 'equal',
  },
  {
    name: 'title and description props',
    config: { titleProp: true, descProp: true },
    source:
      '<svg viewBox="0 0 24 24"><title>Title</title><desc>Description</desc><path d="M0 0h24v24H0z" /></svg>',
    // SVGR's SVGO 3 preset removes <title>; Svgr2's SVGO 4 preset preserves it.
    svgoParity: 'different',
  },
];

describe('SVGR and Svgr2 optimizer output', () => {
  it.each(optimizerCases)(
    '$name',
    async ({ config = {}, source, state, svgoParity }) => {
      const resolvedState = getState(state);
      const [svgr, svgr2SvgoOutput, svgr2OxvgOutput] = await Promise.all([
        transformWithSvgr(
          source,
          {
            ...config,
            plugins: [svgrSvgo, svgrJsx],
            svgo: true,
          } as SvgrConfig,
          resolvedState,
        ),
        transformWithSvgr2(
          source,
          {
            ...config,
            plugins: [svgr2Svgo, svgr2Jsx],
          } as Svgr2Config,
          resolvedState,
        ),
        transformWithSvgr2(
          source,
          {
            ...config,
            plugins: [svgr2Oxvg, svgr2Jsx],
          } as Svgr2Config,
          resolvedState,
        ),
      ]);

      const canonicalSvgr = canonicalizeModule(svgr);
      const canonicalSvgr2Svgo = canonicalizeModule(svgr2SvgoOutput);
      canonicalizeModule(svgr2OxvgOutput);

      if (svgoParity === 'equal') {
        expect(canonicalSvgr2Svgo).toBe(canonicalSvgr);
      } else {
        expect(canonicalSvgr2Svgo).not.toBe(canonicalSvgr);
      }

      expect({
        svgr,
        svgr2Oxvg: svgr2OxvgOutput,
        svgr2Svgo: svgr2SvgoOutput,
      }).toMatchSnapshot();
    },
  );
});
