import { parse as parsePath } from 'path';
import camelCase from 'camelcase';
import type { ConfigPlugin } from './plugins';

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
  const pascalCaseFileName = camelCase(
    parsePath(filePath).name.replace(VALID_CHAR_REGEX, ''),
    {
      pascalCase: true,
    },
  );
  return `Svg${pascalCaseFileName}`;
}

export function expandState(state: Partial<State>): State {
  return {
    componentName: state.componentName || getComponentName(state.filePath),
    ...state,
  };
}
