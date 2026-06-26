import { getOxvgConfig } from './config';

const state = { componentName: 'Icon' };

describe('#getOxvgConfig', () => {
  describe('with no specific config', () => {
    it('returns config with `prefixIds: true`', async () => {
      const config = {};
      expect(await getOxvgConfig(config, state)).toEqual({
        plugins: [
          {
            name: 'preset-default',
            params: { overrides: {} },
          },
          'prefixIds',
        ],
      });
    });
  });

  describe('with `config.icons` enabled', () => {
    it('returns config with `removeViewBox: false`', async () => {
      const config = { icon: true };
      expect(await getOxvgConfig(config, state)).toEqual({
        plugins: [
          {
            name: 'preset-default',
            params: { overrides: { removeViewBox: false } },
          },
          'prefixIds',
        ],
      });
    });
  });

  describe('with `config.dimensions` disabled', () => {
    it('returns config with `removeViewBox: false`', async () => {
      const config = { dimensions: false };
      expect(await getOxvgConfig(config, state)).toEqual({
        plugins: [
          {
            name: 'preset-default',
            params: { overrides: { removeViewBox: false } },
          },
          'prefixIds',
        ],
      });
    });
  });
});
