import { getPlugins, resolvePlugin } from './plugins';
import type { State } from './state';
import type { Config } from './config';

describe('#getPlugins', () => {
  const state: Partial<State> = {
    caller: { defaultPlugins: ['from-state-plugin'] },
  };
  const config: Config = { plugins: ['from-config'] };

  it('should use config if plugins are specified in', () => {
    expect(getPlugins(config, state)).toEqual(['from-config']);
  });

  it('should use caller.defaultPlugins in second choice', () => {
    expect(getPlugins({}, state)).toEqual(['from-state-plugin']);
  });

  it('should default to []', () => {
    expect(getPlugins({}, {})).toEqual([]);
  });

  it('should support caller with "defaultPlugins" in second choice', () => {
    expect(getPlugins({}, { caller: {} })).toEqual([]);
  });
});

describe('#resolvePlugin', () => {
  it('should use function', () => {
    const customPlugin = () => '';

    expect(resolvePlugin(customPlugin)).toBe(customPlugin);
  });

  it('should throw if not found', () => {
    expect(() => resolvePlugin('not-found-plugin')).toThrow(
      'Module "not-found-plugin" missing. Maybe `npm install not-found-plugin` could help!',
    );
  });

  it('should load plugin', () => {
    const plugin = resolvePlugin('./__fixtures__/plugin.cjs');
    expect(plugin('code', {}, { componentName: 'Icon' })).toBe('code fixture');
  });
});
