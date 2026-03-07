import type { TranscriptionContext } from './premium';

export interface ContextPreset {
  key: string;
  name: string;
  icon: string;
  color: string;
  description: string;
  language: string;
  provider: TranscriptionContext['provider'];
  postProcessing: {
    enabled: boolean;
    llm_provider: 'open-ai' | 'anthropic' | 'groq-llm';
    system_prompt: string;
  };
}

export const CONTEXT_PRESETS: ContextPreset[] = [
  {
    key: 'email',
    name: 'Email',
    icon: '\u2709\uFE0F',
    color: '#3b82f6',
    description: 'Professional emails with proper formatting',
    language: 'auto',
    provider: 'local',
    postProcessing: {
      enabled: true,
      llm_provider: 'groq-llm',
      system_prompt:
        'Format this transcription as a professional email. Fix grammar and punctuation. ' +
        'Structure with proper greeting, body paragraphs, and sign-off. ' +
        'Keep the original intent and tone. Output only the formatted email text.',
    },
  },
  {
    key: 'developer',
    name: 'Development',
    icon: '\uD83D\uDCBB',
    color: '#10b981',
    description: 'Code comments, docs, commit messages',
    language: 'en',
    provider: 'local',
    postProcessing: {
      enabled: true,
      llm_provider: 'groq-llm',
      system_prompt:
        'Clean up this transcription for a software development context. ' +
        'Fix technical terminology, format code references with backticks, ' +
        'and structure as clear, concise technical writing. ' +
        'Suitable for commit messages, code comments, or documentation. Output only the cleaned text.',
    },
  },
  {
    key: 'writing',
    name: 'Writing & Content',
    icon: '\u270D\uFE0F',
    color: '#8b5cf6',
    description: 'Blog posts, articles, creative writing',
    language: 'auto',
    provider: 'local',
    postProcessing: {
      enabled: true,
      llm_provider: 'groq-llm',
      system_prompt:
        'Polish this transcription into well-written prose. Fix grammar, improve sentence flow, ' +
        'and organize into coherent paragraphs. Maintain the author\'s voice and ideas. ' +
        'Remove filler words and verbal tics. Output only the polished text.',
    },
  },
  {
    key: 'professional',
    name: 'Professional',
    icon: '\uD83D\uDCBC',
    color: '#f59e0b',
    description: 'Meeting notes, business communication',
    language: 'auto',
    provider: 'local',
    postProcessing: {
      enabled: true,
      llm_provider: 'groq-llm',
      system_prompt:
        'Format this transcription for a professional business context. ' +
        'Use formal tone, fix grammar, and structure clearly. ' +
        'If it sounds like meeting notes, use bullet points. ' +
        'Remove filler words and hesitations. Output only the formatted text.',
    },
  },
  {
    key: 'casual',
    name: 'Casual',
    icon: '\uD83D\uDCAC',
    color: '#ec4899',
    description: 'Friendly messages, chat, social media',
    language: 'auto',
    provider: 'local',
    postProcessing: {
      enabled: true,
      llm_provider: 'groq-llm',
      system_prompt:
        'Clean up this transcription for casual messaging. Keep a friendly, informal tone. ' +
        'Fix obvious errors but preserve the natural conversational style. ' +
        'Remove filler words. Output only the cleaned text.',
    },
  },
];
