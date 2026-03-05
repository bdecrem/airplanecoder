// Agent loop — drives the Qwen model through tool-use cycles

import { chat, chatStream, type Message, type ToolCall, type ToolDef } from './llm/ollama.js';
import { getToolDefinitions, getTool } from './tools/index.js';

const SYSTEM_PROMPT = `You are Airplane Coder, a local coding assistant that runs entirely on-device.
You help users with software engineering tasks: reading code, writing code, debugging, refactoring, running tests, and more.

You have access to tools for file I/O, shell commands, and code search. Use them to accomplish tasks.

Guidelines:
- Read files before modifying them
- Use edit_file for targeted changes, write_file for new files
- Run tests after making changes when appropriate
- Be concise — lead with actions, not explanations
- When searching code, use grep for content and glob for file paths`;

export interface AgentCallbacks {
  onText: (text: string) => void;
  onToolCall: (name: string, args: Record<string, unknown>) => void;
  onToolResult: (name: string, result: string) => void;
  onDone: () => void;
  onError: (error: string) => void;
}

export interface AgentState {
  model: string;
  messages: Message[];
  cwd: string;
}

export function createAgent(model: string, cwd: string): AgentState {
  return {
    model,
    messages: [{ role: 'system', content: SYSTEM_PROMPT }],
    cwd,
  };
}

export async function runAgent(
  state: AgentState,
  userMessage: string,
  callbacks: AgentCallbacks,
): Promise<void> {
  state.messages.push({ role: 'user', content: userMessage });

  const tools = getToolDefinitions();
  let iterations = 0;
  const maxIterations = 20;

  while (iterations < maxIterations) {
    iterations++;

    try {
      // Non-streaming for tool calls (ollama streams don't include tool_calls reliably)
      const response = await chat({
        model: state.model,
        messages: state.messages,
        tools,
      });

      const msg = response.message;

      // Text response
      if (msg.content) {
        callbacks.onText(msg.content);
      }

      // Tool calls
      if (msg.tool_calls && msg.tool_calls.length > 0) {
        state.messages.push(msg);

        for (const tc of msg.tool_calls) {
          const name = tc.function.name;
          let args: Record<string, unknown>;
          try {
            args = JSON.parse(tc.function.arguments);
          } catch {
            args = {};
          }

          callbacks.onToolCall(name, args);

          const tool = getTool(name);
          let result: string;
          if (tool) {
            try {
              result = await tool.execute(args);
            } catch (e: unknown) {
              result = `Error: ${(e as Error).message}`;
            }
          } else {
            result = `Unknown tool: ${name}`;
          }

          callbacks.onToolResult(name, result);

          state.messages.push({
            role: 'tool',
            content: result,
            tool_call_id: tc.id,
          });
        }

        // Continue the loop — model needs to respond to tool results
        continue;
      }

      // No tool calls, no more content — we're done
      state.messages.push(msg);
      break;
    } catch (e: unknown) {
      callbacks.onError((e as Error).message);
      break;
    }
  }

  if (iterations >= maxIterations) {
    callbacks.onError('Agent hit maximum iteration limit (20)');
  }

  callbacks.onDone();
}
