'use strict';

const binding = require('./binding');

function toOptionsJson(options) {
  if (options == null) return undefined;
  if (typeof options === 'string') return options;
  return JSON.stringify(options);
}

function transform(source, options) {
  return binding.transform(source, toOptionsJson(options));
}

module.exports = {
  transform,
};
