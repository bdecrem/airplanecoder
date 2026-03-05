#!/usr/bin/env tsx
// Airplane Coder — Ink-based TUI for local coding agent

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { render, Box, Text, useInput, useApp, useStdout } from 'ink';
import TextInput from 'ink-text-input';
import wrapAnsi from 'wrap-ansi';
import stringWidth from 'string-width';
import { createAgent, runAgent, type AgentState } from './agent.js';
import { isAvailable, listModels } from './llm/ollama.js';

// === Defaults ===
const DEFAULT_MODEL = process.env.AIRPLANE_MODEL || 'qwen2.5-coder:7b';

const SPLASH = `
  ✈  AIRPLANE CODER

  Local coding agent — fully offline
  Powered by Qwen via Ollama

  Type a task to get started.
  /model to switch models, /help for commands.
`;

// === Message types ===
interface DisplayMessage {
  type: 'user' | 'assistant' | 'tool' | 'result' | 'system' | 'info';
  text: string;
}

function getMessageStyle(type: string) {
  switch (type) {
    case 'user': return { dimColor: true, prefix: '> ' };
    case 'assistant': return { prefix: '  ' };
    case 'tool': return { color: 'cyan' as const, prefix: '  ⚡ ' };
    case 'result': return { color: 'gray' as const, prefix: '     ' };
    case 'system': return { color: 'yellow' as const, prefix: '' };
    case 'info': return { prefix: '' };
    default: return { prefix: '' };
  }
}

function getVisualLineCount(msg: DisplayMessage, width: number): number {
  const style = getMessageStyle(msg.type);
  const prefixWidth = stringWidth(style.prefix);
  const contentWidth = Math.max(20, width - prefixWidth);
  const wrapped = wrapAnsi(msg.text, contentWidth, { hard: true });
  return wrapped.split('\n').length;
}

// === Hairline ===
function Hairline() {
  const { stdout } = useStdout();
  const width = stdout?.columns || 80;
  return <Text dimColor>{'─'.repeat(width)}</Text>;
}

// === Messages ===
function Messages({ messages, maxHeight, width, scrollOffset }: {
  messages: DisplayMessage[];
  maxHeight: number;
  width: number;
  scrollOffset: number;
}) {
  const lineCounts = messages.map(msg => getVisualLineCount(msg, width));
  const totalLines = lineCounts.reduce((a, b) => a + b, 0);
  const maxScrollOffset = Math.max(0, totalLines - maxHeight);
  const effectiveOffset = Math.min(scrollOffset, maxScrollOffset);

  let linesFromEnd = effectiveOffset;
  let endIndex = messages.length;

  for (let i = messages.length - 1; i >= 0 && linesFromEnd > 0; i--) {
    if (linesFromEnd >= lineCounts[i]) {
      linesFromEnd -= lineCounts[i];
      endIndex = i;
    } else {
      break;
    }
  }

  let visibleLines = 0;
  let startIndex = endIndex;

  for (let i = endIndex - 1; i >= 0; i--) {
    if (visibleLines + lineCounts[i] <= maxHeight) {
      visibleLines += lineCounts[i];
      startIndex = i;
    } else {
      break;
    }
  }

  const visibleMessages = messages.slice(startIndex, endIndex);
  const hasMoreAbove = startIndex > 0;
  const hasMoreBelow = effectiveOffset > 0;

  return (
    <Box flexDirection="column" flexGrow={1}>
      {hasMoreAbove && (
        <Text dimColor>  ↑ {startIndex} older messages</Text>
      )}
      {visibleMessages.map((msg, i) => (
        <MessageLine key={startIndex + i} message={msg} width={width} />
      ))}
      {hasMoreBelow && (
        <Text dimColor>  ↓ scroll down for recent</Text>
      )}
    </Box>
  );
}

function MessageLine({ message, width }: { message: DisplayMessage; width: number }) {
  const style = getMessageStyle(message.type);
  const prefixWidth = stringWidth(style.prefix);
  const contentWidth = Math.max(20, width - prefixWidth);
  const wrapped = wrapAnsi(message.text, contentWidth, { hard: true });
  const lines = wrapped.split('\n');
  const indent = ' '.repeat(prefixWidth);
  const { prefix, ...textStyle } = style;

  return (
    <Box flexDirection="column">
      {lines.map((line, i) => (
        <Text key={i} {...textStyle}>{i === 0 ? prefix : indent}{line}</Text>
      ))}
    </Box>
  );
}

// === Input Bar ===
function InputBar({ value, onChange, onSubmit, isProcessing }: {
  value: string;
  onChange: (v: string) => void;
  onSubmit: (v: string) => void;
  isProcessing: boolean;
}) {
  return (
    <Box>
      <Text color="green">&gt; </Text>
      {isProcessing ? (
        <Text dimColor>thinking...</Text>
      ) : (
        <TextInput value={value} onChange={onChange} onSubmit={onSubmit} placeholder="" />
      )}
    </Box>
  );
}

// === Status Bar ===
function StatusBar({ model, cwd }: { model: string; cwd: string }) {
  const shortCwd = cwd.replace(process.env.HOME || '', '~');
  return (
    <Box>
      <Text dimColor>{model} | {shortCwd}</Text>
    </Box>
  );
}

// === Main App ===
function App() {
  const { exit } = useApp();
  const { stdout } = useStdout();

  const [input, setInput] = useState('');
  const [messages, setMessages] = useState<DisplayMessage[]>([]);
  const [isProcessing, setIsProcessing] = useState(false);
  const [showSplash, setShowSplash] = useState(true);
  const [scrollOffset, setScrollOffset] = useState(0);
  const [model, setModel] = useState(DEFAULT_MODEL);
  const [ollamaOk, setOllamaOk] = useState<boolean | null>(null);

  const agentRef = useRef<AgentState | null>(null);

  const terminalHeight = stdout?.rows || 24;
  const reservedLines = 4;
  const maxMessageHeight = Math.max(5, terminalHeight - reservedLines);
  const termWidth = stdout?.columns || 80;

  // Check ollama on mount
  useEffect(() => {
    isAvailable().then(ok => {
      setOllamaOk(ok);
      if (!ok) {
        addMessage('system', 'Ollama not running. Start it with: ollama serve');
      }
    });
  }, []);

  // Init agent
  useEffect(() => {
    agentRef.current = createAgent(model, process.cwd());
  }, [model]);

  const addMessage = useCallback((type: DisplayMessage['type'], text: string) => {
    setMessages(prev => [...prev, { type, text }]);
    setScrollOffset(0);
  }, []);

  // Keyboard handling
  useInput((char, key) => {
    if (isProcessing) return;
    if (key.shift && key.upArrow) { setScrollOffset(p => p + 3); return; }
    if (key.shift && key.downArrow) { setScrollOffset(p => Math.max(0, p - 3)); return; }
    if (key.pageUp) { setScrollOffset(p => p + maxMessageHeight); return; }
    if (key.pageDown) { setScrollOffset(p => Math.max(0, p - maxMessageHeight)); return; }
    if (key.ctrl && char === 'c') exit();
  });

  const handleSubmit = useCallback(async (value: string) => {
    const trimmed = value.trim();
    if (!trimmed) return;

    setInput('');
    setShowSplash(false);

    // Slash commands
    if (trimmed.startsWith('/')) {
      const parts = trimmed.split(' ');
      const cmd = parts[0].toLowerCase();

      switch (cmd) {
        case '/exit':
          exit();
          return;
        case '/clear':
          setMessages([]);
          agentRef.current = createAgent(model, process.cwd());
          addMessage('system', 'Conversation cleared.');
          return;
        case '/model': {
          const newModel = parts[1];
          if (!newModel) {
            const models = await listModels();
            addMessage('info', `Current: ${model}\nAvailable:\n${models.map(m => `  ${m}`).join('\n')}`);
          } else {
            setModel(newModel);
            agentRef.current = createAgent(newModel, process.cwd());
            addMessage('system', `Switched to ${newModel}`);
          }
          return;
        }
        case '/help':
          addMessage('info', [
            'Commands:',
            '  /model [name]  — Show or switch model',
            '  /clear         — Reset conversation',
            '  /exit          — Quit',
            '  /help          — This message',
            '',
            'Tools available: read_file, write_file, edit_file, shell, grep, glob',
          ].join('\n'));
          return;
        default:
          addMessage('system', `Unknown command: ${cmd}`);
          return;
      }
    }

    if (!ollamaOk) {
      addMessage('system', 'Ollama not running. Start it with: ollama serve');
      return;
    }

    if (!agentRef.current) return;

    addMessage('user', trimmed);
    setIsProcessing(true);

    try {
      await runAgent(agentRef.current, trimmed, {
        onText: (text) => addMessage('assistant', text),
        onToolCall: (name, args) => {
          const summary = formatToolCall(name, args);
          addMessage('tool', summary);
        },
        onToolResult: (name, result) => {
          // Truncate long results for display
          const display = result.length > 500
            ? result.slice(0, 500) + `\n... (${result.length} chars total)`
            : result;
          addMessage('result', display);
        },
        onDone: () => {},
        onError: (err) => addMessage('system', `Error: ${err}`),
      });
    } catch (e: unknown) {
      addMessage('system', `Error: ${(e as Error).message}`);
    }

    setIsProcessing(false);
  }, [model, ollamaOk, exit, addMessage]);

  if (ollamaOk === null) {
    return <Text dimColor>Checking ollama...</Text>;
  }

  return (
    <Box flexDirection="column" height={terminalHeight}>
      <Box flexDirection="column" height={maxMessageHeight} overflowY="hidden">
        {showSplash ? (
          <Text>{SPLASH}</Text>
        ) : (
          <Messages messages={messages} maxHeight={maxMessageHeight} width={termWidth} scrollOffset={scrollOffset} />
        )}
      </Box>

      <Hairline />

      <InputBar
        value={input}
        onChange={setInput}
        onSubmit={handleSubmit}
        isProcessing={isProcessing}
      />

      <Hairline />

      <StatusBar model={model} cwd={process.cwd()} />
    </Box>
  );
}

// Format a tool call for display
function formatToolCall(name: string, args: Record<string, unknown>): string {
  switch (name) {
    case 'read_file': return `read ${args.path}`;
    case 'write_file': return `write ${args.path}`;
    case 'edit_file': return `edit ${args.path}`;
    case 'shell': return `$ ${args.command}`;
    case 'grep': return `grep "${args.pattern}"${args.path ? ` in ${args.path}` : ''}`;
    case 'glob': return `glob ${args.pattern}`;
    default: return `${name}(${JSON.stringify(args)})`;
  }
}

// === Start ===
render(<App />);
