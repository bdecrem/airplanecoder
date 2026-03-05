import { exec } from 'child_process';
import type { Tool } from './index.js';

const TIMEOUT_MS = 30_000;

export const shellTool: Tool = {
  definition: {
    type: 'function',
    function: {
      name: 'shell',
      description: 'Execute a shell command and return its output. Use for running tests, git commands, builds, etc. Times out after 30 seconds.',
      parameters: {
        type: 'object',
        properties: {
          command: { type: 'string', description: 'Shell command to execute' },
          cwd: { type: 'string', description: 'Working directory (optional)' },
        },
        required: ['command'],
      },
    },
  },

  async execute(args) {
    const command = args.command as string;
    const cwd = (args.cwd as string) || process.cwd();

    return new Promise<string>((resolve) => {
      exec(command, { cwd, timeout: TIMEOUT_MS, maxBuffer: 1024 * 1024 }, (error, stdout, stderr) => {
        let result = '';
        if (stdout) result += stdout;
        if (stderr) result += (result ? '\n' : '') + stderr;
        if (error && !stdout && !stderr) {
          result = `Error: ${error.message}`;
        }
        resolve(result || '(no output)');
      });
    });
  },
};
