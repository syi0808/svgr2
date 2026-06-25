export type ExpandProps = boolean | 'start' | 'end';
export type Icon = boolean | string | number;
export type ExportType = 'default' | 'named';
export type JsxRuntime = 'classic' | 'classic-preact' | 'automatic';

export interface JsxRuntimeImport {
  source: string;
  namespace?: string;
  defaultSpecifier?: string;
  specifiers?: string[];
}

export interface TransformOptions {
  componentName?: string;
  previousExport?: string;
  ref?: boolean;
  titleProp?: boolean;
  descProp?: boolean;
  expandProps?: ExpandProps;
  dimensions?: boolean;
  icon?: Icon;
  native?: boolean;
  typescript?: boolean;
  memo?: boolean;
  svgProps?: Record<string, string> | Array<[string, string]>;
  replaceAttrValues?: Record<string, string> | Array<[string, string]>;
  exportType?: ExportType;
  namedExport?: string;
  jsxRuntime?: JsxRuntime;
  jsxRuntimeImport?: JsxRuntimeImport;
  importSource?: string;
}

export function transform(
  source: string,
  options?: TransformOptions,
): string;
