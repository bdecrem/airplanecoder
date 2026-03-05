import { glob as globFn } from 'glob';
import type { Tool } from './index.js';

export const globTool: Tool = {
  definition: {
    type: 'function',
    function: {
      name: 'glob',
      description: 'Find files matching a glob pattern. Returns matching file paths.',
      parameters: {
        type: 'object',
        properties: {
          pattern: { type: 'string', description: 'Glob pattern, e.g. "src/**/*.ts" or "*.json"' },
          path: { type: 'string', description: 'Base directory to search in (default: current dir)' },
        },
        required: ['pattern'],
      },
    },
  },

  async execute(args) {
    const pattern = args.pattern as string;
    const cwd = (args.path as string) || process.cwd();

    const files = await globFn(pattern, {
      cwd,
      ignore: ['node_modules/**', '.git/**', 'dist/**'],
      absolute: true,
    });

    if (files.length === 0) return 'No files matched.';

    files.sort();
    if (files.length > 200) {
      return files.slice(0, 200).join('\n') + `\n... (${files.length - 200} more)`;
    }
    return files.join('\n');
  },
};
