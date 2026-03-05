import { writeFile, mkdir } from 'fs/promises';
import { dirname } from 'path';
import type { Tool } from './index.js';

export const writeTool: Tool = {
  definition: {
    type: 'function',
    function: {
      name: 'write_file',
      description: 'Write content to a file. Creates parent directories if needed. Overwrites existing files.',
      parameters: {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'File path to write' },
          content: { type: 'string', description: 'Content to write' },
        },
        required: ['path', 'content'],
      },
    },
  },

  async execute(args) {
    const path = args.path as string;
    const content = args.content as string;

    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, content, 'utf-8');

    const lines = content.split('\n').length;
    return `Wrote ${lines} lines to ${path}`;
  },
};
