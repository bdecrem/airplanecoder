import { exec } from 'child_process';
import type { Tool } from './index.js';

export const grepTool: Tool = {
  definition: {
    type: 'function',
    function: {
      name: 'grep',
      description: 'Search file contents for a pattern. Returns matching lines with file paths and line numbers.',
      parameters: {
        type: 'object',
        properties: {
          pattern: { type: 'string', description: 'Search pattern (regex supported)' },
          path: { type: 'string', description: 'Directory or file to search in (default: current dir)' },
          include: { type: 'string', description: 'File glob pattern, e.g. "*.ts" (optional)' },
        },
        required: ['pattern'],
      },
    },
  },

  async execute(args) {
    const pattern = args.pattern as string;
    const path = (args.path as string) || '.';
    const include = args.include as string | undefined;

    let cmd = `grep -rn --color=never`;
    if (include) cmd += ` --include="${include}"`;
    cmd += ` "${pattern.replace(/"/g, '\\"')}" "${path}"`;

    // Exclude common noise
    cmd += ` --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=dist`;

    return new Promise<string>((resolve) => {
      exec(cmd, { maxBuffer: 1024 * 1024, timeout: 10_000 }, (error, stdout) => {
        if (stdout) {
          const lines = stdout.trim().split('\n');
          if (lines.length > 100) {
            resolve(lines.slice(0, 100).join('\n') + `\n... (${lines.length - 100} more matches)`);
          } else {
            resolve(stdout.trim());
          }
        } else {
          resolve('No matches found.');
        }
      });
    });
  },
};
