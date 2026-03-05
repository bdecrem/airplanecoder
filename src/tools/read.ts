import { readFile } from 'fs/promises';
import { existsSync } from 'fs';
import type { Tool } from './index.js';

export const readTool: Tool = {
  definition: {
    type: 'function',
    function: {
      name: 'read_file',
      description: 'Read the contents of a file. Returns the file content with line numbers.',
      parameters: {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'Absolute or relative file path' },
          offset: { type: 'number', description: 'Start line (1-based, optional)' },
          limit: { type: 'number', description: 'Max lines to read (optional)' },
        },
        required: ['path'],
      },
    },
  },

  async execute(args) {
    const path = args.path as string;
    const offset = (args.offset as number) || 1;
    const limit = args.limit as number | undefined;

    if (!existsSync(path)) {
      return `Error: file not found: ${path}`;
    }

    const content = await readFile(path, 'utf-8');
    const lines = content.split('\n');
    const start = Math.max(0, offset - 1);
    const end = limit ? start + limit : lines.length;
    const slice = lines.slice(start, end);

    return slice
      .map((line, i) => `${String(start + i + 1).padStart(5)} | ${line}`)
      .join('\n');
  },
};
