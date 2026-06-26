import { parse as parsePath } from 'path';
import type { ConfigPlugin } from './plugins';
import { pascalCase } from 'change-case';

export interface State {
  filePath?: string;
  componentName: string;
  caller?: {
    name?: string;
    previousExport?: string | null;
    defaultPlugins?: ConfigPlugin[];
  };
}

const VALID_CHAR_REGEX = /[^a-zA-Z0-9 _-]/g;

function getComponentName(filePath?: string): string {
  if (!filePath) return 'SvgComponent';
  const pascalCaseFileName = pascalCase(
    parsePath(filePath).name.replace(VALID_CHAR_REGEX, ''),
  );
  return `Svg${pascalCaseFileName}`;
}

export function expandState(state: Partial<State>): State {
  return {
    ...state,
    componentName: state.componentName || getComponentName(state.filePath),
  };
}
