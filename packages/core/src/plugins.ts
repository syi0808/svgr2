import { Config } from './config.js';
import type { State } from './state.js';

export interface Plugin<PluginOption = Record<PropertyKey, unknown>> {
  (code: string, config: Config, state: State, options?: Partial<PluginOption>): string;
}

export type ConfigPlugin = string | Plugin;

const DEFAULT_PLUGINS: Plugin[] = [];

export function getPlugins(
  config: Config,
  state: Partial<State>,
): ConfigPlugin[] {
  if (config.plugins) {
    return config.plugins;
  }

  if (state.caller?.defaultPlugins) {
    return state.caller.defaultPlugins;
  }

  return DEFAULT_PLUGINS;
}

export function resolvePlugin(plugin: ConfigPlugin): Plugin {
  if (typeof plugin === 'function') {
    return plugin;
  }

  if (typeof plugin === 'string') {
    return loadPlugin(plugin);
  }

  throw new Error(`Invalid plugin "${plugin}"`);
}

const pluginCache: Record<string, Plugin> = {};

const resolveModule = (m: any) => (m ? m.default || m : null);

export function loadPlugin(moduleName: string): Plugin {
  if (pluginCache[moduleName]) {
    return pluginCache[moduleName];
  }

  try {
    const plugin = resolveModule(require(moduleName));

    if (!plugin) {
      throw new Error(`Invalid plugin "${moduleName}"`);
    }

    pluginCache[moduleName] = plugin;

    // @ts-expect-error cause record can not caught cache exist
    return pluginCache[moduleName];
  } catch (error) {
    console.log(error);
    throw new Error(
      `Module "${moduleName}" missing. Maybe \`npm install ${moduleName}\` could help!`,
    );
  }
}
