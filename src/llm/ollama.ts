// Ollama client — direct HTTP, no SDK needed

export interface Message {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string;
  tool_calls?: ToolCall[];
  tool_call_id?: string;
}

export interface ToolCall {
  id: string;
  type: 'function';
  function: {
    name: string;
    arguments: string; // JSON string
  };
}

export interface ToolDef {
  type: 'function';
  function: {
    name: string;
    description: string;
    parameters: Record<string, unknown>;
  };
}

export interface ChatOptions {
  model: string;
  messages: Message[];
  tools?: ToolDef[];
  stream?: boolean;
  temperature?: number;
}

export interface ChatResponse {
  message: Message;
  done: boolean;
  total_duration?: number;
  eval_count?: number;
}

export interface StreamChunk {
  message: { role: string; content: string; tool_calls?: ToolCall[] };
  done: boolean;
}

const OLLAMA_BASE = process.env.OLLAMA_HOST || 'http://localhost:11434';

export async function chat(options: ChatOptions): Promise<ChatResponse> {
  const res = await fetch(`${OLLAMA_BASE}/api/chat`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: options.model,
      messages: options.messages,
      tools: options.tools,
      stream: false,
      options: {
        temperature: options.temperature ?? 0.1,
        num_ctx: 8192,
      },
    }),
  });

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Ollama error ${res.status}: ${text}`);
  }

  return res.json() as Promise<ChatResponse>;
}

export async function* chatStream(options: ChatOptions): AsyncGenerator<StreamChunk> {
  const res = await fetch(`${OLLAMA_BASE}/api/chat`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: options.model,
      messages: options.messages,
      tools: options.tools,
      stream: true,
      options: {
        temperature: options.temperature ?? 0.1,
        num_ctx: 8192,
      },
    }),
  });

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Ollama error ${res.status}: ${text}`);
  }

  const reader = res.body?.getReader();
  if (!reader) throw new Error('No response body');

  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop() || '';

    for (const line of lines) {
      if (!line.trim()) continue;
      yield JSON.parse(line) as StreamChunk;
    }
  }

  if (buffer.trim()) {
    yield JSON.parse(buffer) as StreamChunk;
  }
}

export async function listModels(): Promise<string[]> {
  const res = await fetch(`${OLLAMA_BASE}/api/tags`);
  if (!res.ok) return [];
  const data = await res.json() as { models: { name: string }[] };
  return data.models.map(m => m.name);
}

// Check if ollama is running
export async function isAvailable(): Promise<boolean> {
  try {
    const res = await fetch(`${OLLAMA_BASE}/api/tags`);
    return res.ok;
  } catch {
    return false;
  }
}
