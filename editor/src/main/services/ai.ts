import { ipcMain, BrowserWindow } from 'electron';
import Anthropic from '@anthropic-ai/sdk';
import OpenAI from 'openai';

// Store API keys in memory (later: keytar for OS keychain)
const apiKeys: Record<string, string> = {};

let anthropicClient: Anthropic | null = null;
let openaiClient: OpenAI | null = null;

function getAnthropicClient(): Anthropic | null {
  const key = apiKeys['anthropic'];
  if (!key) return null;
  if (!anthropicClient) {
    anthropicClient = new Anthropic({ apiKey: key });
  }
  return anthropicClient;
}

function getOpenAIClient(): OpenAI | null {
  const key = apiKeys['openai'];
  if (!key) return null;
  if (!openaiClient) {
    openaiClient = new OpenAI({ apiKey: key });
  }
  return openaiClient;
}

// Called by connections service to sync API keys
export function setAIKey(provider: string, key: string): void {
  apiKeys[provider] = key;
  if (provider === 'anthropic') anthropicClient = null;
  if (provider === 'openai') openaiClient = null;
}

export function registerAIHandlers() {
  // Set API key
  ipcMain.handle('ai:setKey', (_event, provider: string, key: string) => {
    apiKeys[provider] = key;
    // Reset client so it picks up new key
    if (provider === 'anthropic') anthropicClient = null;
    if (provider === 'openai') openaiClient = null;
    return true;
  });

  // Get stored API key (masked)
  ipcMain.handle('ai:getKey', (_event, provider: string) => {
    const key = apiKeys[provider];
    if (!key) return null;
    return key.substring(0, 8) + '...' + key.substring(key.length - 4);
  });

  // Check if key is set
  ipcMain.handle('ai:hasKey', (_event, provider: string) => {
    return !!apiKeys[provider];
  });

  // Chat with Claude (streaming)
  ipcMain.handle('ai:chatClaude', async (_event, messages: { role: string; content: string }[], model?: string) => {
    const client = getAnthropicClient();
    if (!client) return { error: 'API 키가 설정되지 않았습니다.' };

    const win = BrowserWindow.getFocusedWindow();

    try {
      const stream = await client.messages.stream({
        model: model || 'claude-sonnet-4-20250514',
        max_tokens: 4096,
        messages: messages.map(m => ({
          role: m.role as 'user' | 'assistant',
          content: m.content,
        })),
        system: `You are VidEplace AI, an intelligent coding assistant built into the VidEplace IDE.
You help users create, modify, and debug code.
When generating code, always provide complete, working code with file paths.
Respond in the same language the user uses (Korean or English).
Format code blocks with the file path as the language hint.`,
      });

      let fullText = '';
      let inputTokens = 0;
      let outputTokens = 0;

      stream.on('text', (text) => {
        fullText += text;
        win?.webContents.send('ai:stream', text);
      });

      const finalMessage = await stream.finalMessage();
      inputTokens = finalMessage.usage.input_tokens;
      outputTokens = finalMessage.usage.output_tokens;

      win?.webContents.send('ai:streamEnd');

      return {
        content: fullText,
        inputTokens,
        outputTokens,
        model: finalMessage.model,
      };
    } catch (err: any) {
      return { error: err.message || 'Claude API 오류' };
    }
  });

  // Chat with OpenAI (streaming)
  ipcMain.handle('ai:chatOpenAI', async (_event, messages: { role: string; content: string }[], model?: string) => {
    const client = getOpenAIClient();
    if (!client) return { error: 'API 키가 설정되지 않았습니다.' };

    const win = BrowserWindow.getFocusedWindow();

    try {
      const stream = await client.chat.completions.create({
        model: model || 'gpt-4o',
        messages: [
          {
            role: 'system',
            content: `You are VidEplace AI, an intelligent coding assistant built into the VidEplace IDE.
You help users create, modify, and debug code.
When generating code, always provide complete, working code with file paths.
Respond in the same language the user uses (Korean or English).`,
          },
          ...messages.map(m => ({
            role: m.role as 'user' | 'assistant',
            content: m.content,
          })),
        ],
        stream: true,
      });

      let fullText = '';

      for await (const chunk of stream) {
        const text = chunk.choices[0]?.delta?.content || '';
        if (text) {
          fullText += text;
          win?.webContents.send('ai:stream', text);
        }
      }

      win?.webContents.send('ai:streamEnd');

      return {
        content: fullText,
        model: model || 'gpt-4o',
      };
    } catch (err: any) {
      return { error: err.message || 'OpenAI API 오류' };
    }
  });
}
