import { getOxvgConfig } from './config';

describe('#getOxvgConfig', () => {
  describe('with no specific config', () => {
    it('enables viewBox removal and ID prefixing', () => {
      const config = {};
      const result = getOxvgConfig(config);

      expect({
        prefixIds: result.prefixIds,
        removeViewBox: result.removeViewBox,
      }).toEqual({
        prefixIds: {
          delim: '__',
          prefix: { type: 'Default' },
          prefixClassNames: true,
          prefixIds: true,
        },
        removeViewBox: { field0: true },
      });
    });
  });

  describe('with `config.icon` enabled', () => {
    it('preserves the viewBox', () => {
      const config = { icon: true };
      expect(getOxvgConfig(config).removeViewBox).toBeUndefined();
    });
  });

  describe('with `config.dimensions` disabled', () => {
    it('preserves the viewBox', () => {
      const config = { dimensions: false };
      expect(getOxvgConfig(config).removeViewBox).toBeUndefined();
    });
  });

  describe('with `config.native` enabled', () => {
    it('inlines all matching styles', () => {
      const config = { native: true };
      expect(getOxvgConfig(config).inlineStyles).toEqual({
        onlyMatchedOnce: false,
        removeMatchedSelectors: true,
        useMqs: ['', 'screen'],
        usePseudos: [''],
      });
    });
  });
});
