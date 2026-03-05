import { readFile, writeFile } from 'fs/promises';
import { existsSync } from 'fs';
import type { Tool } from './index.js';

export const editTool: Tool = {
  definition: {
    type: 'function',
    function: {
      name: 'edit_file',
      description: 'Edit a file by replacing a specific string with new content. The old_string must match exactly (including whitespace). For creating new files, use write_file instead.',
      parameters: {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'File path to edit' },
          old_string: { type: 'string', description: 'Exact string to find and replace' },
          new_string: { type: 'string', description: 'Replacement string' },
        },
        required: ['path', 'old_string', 'new_string'],
      },
    },
  },

  async execute(args) {
    const path = args.path as string;
    const oldStr = args.old_string as string;
    const newStr = args.new_string as string;

    if (!existsSync(path)) {
      return `Error: file not found: ${path}`;
    }

    const content = await readFile(path, 'utf-8');
    const occurrences = content.split(oldStr).length - 1;

    if (occurrences === 0) {
      return `Error: old_string not found in ${path}`;
    }

    if (occurrences > 1) {
      return `Error: old_string found ${occurrences} times in ${path}. Make the match more specific.`;
    }

    const updated = content.replace(oldStr, newStr);
    await writeFile(path, updated, 'utf-8');

    return `Edited ${path} (replaced 1 occurrence)`;
  },
};
