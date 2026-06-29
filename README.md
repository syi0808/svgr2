<h1 align="left">Svgr2</h1>
<p align="center" style="font-size: 1.2rem;">A faster, modern alternative to SVGR.</p>

Svgr2 transforms SVG files into React components. It is an ESM-first fork of
[SVGR](https://github.com/gregberge/svgr) that replaces the core JSX pipeline
with Oxc and Rust-based tooling.

## Performance

Benchmarks were run on an Apple M2 Pro MacBook Pro with a 10-core CPU and 16 GB
of memory, using macOS 26.5.1 and Node.js 24.5.0.

Results are reported in operations per second (`ops/s`). Higher is better.
Values in parentheses show the speedup over SVGR for the same fixture. Each
implementation was sampled for 1.5 seconds after a 0.3-second warmup.

### End-to-end transform

These benchmarks include SVG optimization and JSX generation.

| Fixture                     | SVGR (ops/s) |    Svgr2 `transform` | Svgr2 `createTransformerSync` |
| --------------------------- | -----------: | -------------------: | ----------------------------: |
| Tiny                        |        3,044 | 10,374 ops/s (3.41×) |          17,627 ops/s (5.79×) |
| Many attributes             |           86 |    294 ops/s (3.43×) |             307 ops/s (3.58×) |
| Huge path                   |          321 |    332 ops/s (1.03×) |             337 ops/s (1.05×) |
| Deep nesting                |          236 |    262 ops/s (1.11×) |             266 ops/s (1.13×) |
| Style-heavy                 |           59 |   623 ops/s (10.59×) |            636 ops/s (10.81×) |
| Entity text                 |        2,694 |  9,631 ops/s (3.58×) |          15,621 ops/s (5.80×) |
| Replace attribute values    |          104 |    277 ops/s (2.67×) |             281 ops/s (2.71×) |
| React Native                |        1,421 |  5,893 ops/s (4.15×) |           7,821 ops/s (5.50×) |
| Title and description props |        2,022 |  9,568 ops/s (4.73×) |          15,324 ops/s (7.58×) |

The `huge path` result depends heavily on OXVG's path optimizer. Further
improvements to this case depend on
[OXVG pull request #235](https://github.com/noahbald/oxvg/pull/235), a draft
rewrite of its path optimization logic that still lists performance regressions
as work in progress.

### JSX generation only

These benchmarks disable SVG optimization and compare the JSX pipelines
directly.

| Fixture            | SVGR (ops/s) |      Svgr2 `transform` | Svgr2 `createTransformerSync` |
| ------------------ | -----------: | ---------------------: | ----------------------------: |
| Tiny               |        4,228 | 126,807 ops/s (29.99×) |        150,772 ops/s (35.66×) |
| Many attributes    |          110 |   2,984 ops/s (27.14×) |          3,003 ops/s (27.31×) |
| Huge path          |        2,248 |   10,928 ops/s (4.86×) |          11,290 ops/s (5.02×) |
| Deep nesting       |          455 |  10,760 ops/s (23.67×) |         11,073 ops/s (24.36×) |
| Style-heavy        |           99 |   3,413 ops/s (34.46×) |          3,422 ops/s (34.55×) |
| Entity text        |        3,520 | 109,102 ops/s (31.00×) |        129,204 ops/s (36.71×) |
| Comments and CDATA |        3,550 | 102,884 ops/s (28.98×) |        121,014 ops/s (34.09×) |

`createTransformerSync` loads the configuration and plugins once, then reuses
them across transforms.

Results were measured with the fixtures in [`bench`](./bench):

```sh
pnpm build
pnpm --filter ./bench bench --run
```

Benchmark results vary by machine and runtime. Run the suite in your own
environment before making performance-sensitive decisions.

## Features

- ESM-first packages targeting Node.js 22 and later.
- JSX generation powered by Oxc and Rust.
- SVG optimization with OXVG or SVGO.
- Reusable transformers that avoid repeated configuration and plugin setup.
- The new `createTransformer` function reduces option resolution overhead when
  running multiple transforms in the same JavaScript runtime.

### Breaking changes from SVGR

Plugin-specific top-level configuration options have been removed. Import each
plugin and pass it directly to `plugins` instead:

```ts
import { transform } from '@svgr2/core';
import { jsxPlugin } from '@svgr2/plugin-jsx-oxc';
import { oxvgPlugin } from '@svgr2/plugin-oxvg';

const component = await transform(svgSource, {
  plugins: [oxvgPlugin, jsxPlugin],
});
```

OXVG is the recommended SVG optimizer, with a focus on safe transformations and
fast execution. SVGO remains available through `@svgr2/plugin-svgo`.

The Prettier, webpack, and Rollup plugins maintained by SVGR are not included in
Svgr2. They may be added in the future if there is enough demand.

## License

Licensed under the MIT License. See [LICENSE](./LICENSE) for more information.

## Acknowledgements

Svgr2 began as a fork of [SVGR](https://github.com/gregberge/svgr), created by
[Greg Bergé](https://github.com/gregberge). Most of the project exists because
of his ideas and work. Svgr2 builds on that foundation with a smaller set of
changes.

JSX generation is powered by [Oxc](https://oxc.rs/). Thank you to the Oxc
contributors for building and maintaining it.

### SVGR acknowledgements

SVGR was popularized by
[Christopher Chedeau](https://twitter.com/vjeux) and included in
[Create React App](https://github.com/facebook/create-react-app) thanks to
[Dan Abramov](https://twitter.com/dan_abramov). Thanks also to
[Sven Sauleau](https://twitter.com/svensauleau) for his help and insight.
