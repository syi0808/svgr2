import combined from './index.js';

describe('plugin-combined', () => {
  it('optimises and transforms SVG in one plugin call', () => {
    const result = combined(
      '<svg viewBox="0 0 10 10"><!-- remove me --><path d="M 0 0 L 10 10"/></svg>',
      { icon: true },
      { componentName: 'Icon' },
    );

    expect(result).toMatchSnapshot();
  });

  it('supports the automatic JSX runtime', () => {
    const result = combined(
      '<svg />',
      { jsxRuntime: 'automatic' },
      { componentName: 'Icon' },
    );

    expect(result).toMatchSnapshot();
  });
});
