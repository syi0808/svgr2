import { expandState } from './state';
import { loadConfig, loadConfigSync } from './config';
import { resolvePlugin, getPlugins } from './plugins';
import type { Config } from './config';
import type { State } from './state';

function run(code: string, config: Config, state: Partial<State>): string {
  const expandedState = expandState(state);
  const plugins = getPlugins(config, state).map(resolvePlugin);
  let nextCode = String(code).replace('\0', '');

  for (const plugin of plugins) {
    nextCode = plugin(nextCode, config, expandedState);
  }

  return nextCode;
}

export async function transform(
  code: string,
  config: Config = {},
  state: Partial<State> = {},
): Promise<string> {
  config = await loadConfig(config, state);
  return run(code, config, state);
}

transform.sync = (
  code: string,
  config: Config = {},
  state: Partial<State> = {},
): string => {
  config = loadConfigSync(config, state);
  return run(code, config, state);
};

export function transformSync(
  code: string,
  config: Config = {},
  state: Partial<State> = {},
): string {
  config = loadConfigSync(config, state);
  return run(code, config, state);
}

type Transformer = {
  transform: (code: string) => string;
};

export async function createTransformer(
  config: Config = {},
  state: Partial<State> = {},
): Promise<Transformer> {
  config = await loadConfig(config, state);

  return {
    transform: (code: string) => run(code, config, state),
  };
}

export async function createTransformerSync(
  config: Config = {},
  state: Partial<State> = {},
): Promise<Transformer> {
  config = loadConfigSync(config, state);

  return {
    transform: (code: string) => run(code, config, state),
  };
}
