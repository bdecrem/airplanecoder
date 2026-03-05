// Tool registry — all coding tools available to the agent

import { readTool } from './read.js';
import { writeTool } from './write.js';
import { editTool } from './edit.js';
import { shellTool } from './shell.js';
import { grepTool } from './grep.js';
import { globTool } from './glob.js';
import type { ToolDef } from '../llm/ollama.js';

export interface Tool {
  definition: ToolDef;
  execute: (args: Record<string, unknown>) => Promise<string>;
}

const ALL_TOOLS: Tool[] = [
  readTool,
  writeTool,
  editTool,
  shellTool,
  grepTool,
  globTool,
];

export function getToolDefinitions(): ToolDef[] {
  return ALL_TOOLS.map(t => t.definition);
}

export function getTool(name: string): Tool | undefined {
  return ALL_TOOLS.find(t => t.definition.function.name === name);
}

export { ALL_TOOLS };
