'use strict';

const binding = require('./binding');

function transform(source, options) {
  return binding.transform(source, options);
}

module.exports = {
  transform,
};
