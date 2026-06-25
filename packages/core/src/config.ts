import { cosmiconfig, cosmiconfigSync } from 'cosmiconfig';
import type { Config as SvgoConfig } from 'svgo';
import type { ConfigPlugin } from './plugins';
import type { State } from './state';

export interface Config {
  ref?: boolean;
  titleProp?: boolean;
  descProp?: boolean;
  expandProps?: boolean | 'start' | 'end';
  dimensions?: boolean;
  icon?: boolean | string | number;
  native?: boolean;
  svgProps?: {
    [key: string]: string;
  };
  replaceAttrValues?: {
    [key: string]: string;
  };
  runtimeConfig?: boolean;
  typescript?: boolean;
  svgoConfig?: SvgoConfig;
  configFile?: string;
  memo?: boolean;
  exportType?: 'named' | 'default';
  namedExport?: string;
  jsxRuntime?: 'classic' | 'classic-preact' | 'automatic';
  jsxRuntimeImport?: {
    source: string;
    namespace?: string;
    specifiers?: string[];
    defaultSpecifier?: string;
  };

  // CLI only
  index?: boolean;
  plugins?: ConfigPlugin[];

  // JSX
  jsx?: {};
}

export const DEFAULT_CONFIG: Config = {
  dimensions: true,
  expandProps: 'end',
  icon: false,
  native: false,
  typescript: false,
  memo: false,
  ref: false,
  // replaceAttrValues: undefined,
  // svgProps: undefined,
  // svgoConfig: undefined,
  // template: undefined,
  index: false,
  titleProp: false,
  descProp: false,
  runtimeConfig: true,
  namedExport: 'ReactComponent',
  exportType: 'default',
};

const explorer = cosmiconfig('svgr');
const explorerSync = cosmiconfigSync('svgr');

export async function resolveConfig(
  searchFrom?: string,
  configFile?: string,
): Promise<Config | null> {
  if (configFile == null) {
    const result = await explorer.search(searchFrom);
    return result ? result.config : null;
  }
  const result = await explorer.load(configFile);
  return result ? result.config : null;
}

resolveConfig.sync = (
  searchFrom?: string,
  configFile?: string,
): Config | null => {
  if (configFile == null) {
    const result = explorerSync.search(searchFrom);
    return result ? result.config : null;
  }
  const result = explorerSync.load(configFile);
  return result ? result.config : null;
};

export function resolveConfigSync(
  searchFrom?: string,
  configFile?: string,
): Config | null {
  if (configFile == null) {
    const result = explorerSync.search(searchFrom);
    return result ? result.config : null;
  }
  const result = explorerSync.load(configFile);
  return result ? result.config : null;
}

export async function resolveConfigFile(
  filePath: string,
): Promise<string | null> {
  const result = await explorer.search(filePath);
  return result ? result.filepath : null;
}

resolveConfigFile.sync = (filePath: string): string | null => {
  const result = explorerSync.search(filePath);
  return result ? result.filepath : null;
};

export function resolveConfigFileSync(filePath: string): string | null {
  const result = explorerSync.search(filePath);
  return result ? result.filepath : null;
}

export async function loadConfig(
  { configFile, ...baseConfig }: Config,
  state: Pick<State, 'filePath'> = {},
): Promise<Config> {
  const rcConfig =
    state.filePath && baseConfig.runtimeConfig !== false
      ? await resolveConfig(state.filePath, configFile)
      : {};
  return { ...DEFAULT_CONFIG, ...baseConfig, ...rcConfig };
}

loadConfig.sync = (
  { configFile, ...baseConfig }: Config,
  state: Pick<State, 'filePath'> = {},
): Config => {
  const rcConfig =
    state.filePath && baseConfig.runtimeConfig !== false
      ? resolveConfig.sync(state.filePath, configFile)
      : {};
  return { ...DEFAULT_CONFIG, ...baseConfig, ...rcConfig };
};

export function loadConfigSync(
  { configFile, ...baseConfig }: Config,
  state: Pick<State, 'filePath'> = {},
): Config {
  const rcConfig =
    state.filePath && baseConfig.runtimeConfig !== false
      ? resolveConfig.sync(state.filePath, configFile)
      : {};
  return { ...DEFAULT_CONFIG, ...baseConfig, ...rcConfig };
}
