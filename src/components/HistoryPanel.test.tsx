import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { HistoryPanel } from './HistoryPanel';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

vi.mock('@tauri-apps/api/core');
vi.mock('@tauri-apps/api/event');

const mockHistoryEntries = [
  {
    id: '1',
    text: 'Hello world, this is a test transcription.',
    timestamp: '2024-01-15T10:30:00Z',
    duration_ms: 2500,
    provider: 'whisper.cpp',
    language: 'en',
    audio_path: '/audio/1.wav',
  },
  {
    id: '2',
    text: 'Another transcription entry for testing purposes.',
    timestamp: '2024-01-15T11:00:00Z',
    duration_ms: 3200,
    provider: 'groq',
    language: null,
    audio_path: null,
  },
];

describe('HistoryPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_transcription_history') {
        return Promise.resolve(mockHistoryEntries);
      }
      if (cmd === 'delete_history_entry') {
        return Promise.resolve(undefined);
      }
      if (cmd === 'clear_history') {
        return Promise.resolve(undefined);
      }
      return Promise.resolve(undefined);
    });

    vi.mocked(listen).mockImplementation(() => {
      return Promise.resolve(() => {});
    });

    // Mock clipboard API
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it('should render loading state initially', () => {
    render(<HistoryPanel />);
    // The loading spinner has animate-spin class
    const spinner = document.querySelector('.animate-spin');
    expect(spinner).toBeTruthy();
  });

  it('should render history entries', async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      expect(screen.getByText('Hello world, this is a test transcription.')).toBeInTheDocument();
      expect(screen.getByText('Another transcription entry for testing purposes.')).toBeInTheDocument();
    });
  });

  it('should show empty state when no history', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_transcription_history') {
        return Promise.resolve([]);
      }
      return Promise.resolve(undefined);
    });

    render(<HistoryPanel />);

    await waitFor(() => {
      expect(screen.getByText('No transcriptions yet')).toBeInTheDocument();
    });
  });

  it('should copy text to clipboard when copy button is clicked', async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      expect(screen.getByText('Hello world, this is a test transcription.')).toBeInTheDocument();
    });

    // Hover over the entry to show action buttons
    const entry = screen.getByText('Hello world, this is a test transcription.').closest('.card');
    fireEvent.mouseEnter(entry!);

    const copyButtons = screen.getAllByTitle('Copy to clipboard');
    fireEvent.click(copyButtons[0]);

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('Hello world, this is a test transcription.');
  });

  it('should delete entry when delete button is clicked', async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      expect(screen.getByText('Hello world, this is a test transcription.')).toBeInTheDocument();
    });

    const entry = screen.getByText('Hello world, this is a test transcription.').closest('.card');
    fireEvent.mouseEnter(entry!);

    const deleteButtons = screen.getAllByTitle('Delete');
    fireEvent.click(deleteButtons[0]);

    expect(invoke).toHaveBeenCalledWith('delete_history_entry', { id: '1' });
  });

  it('should show Clear All button when history exists', async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      expect(screen.getByText('Clear All')).toBeInTheDocument();
    });
  });

  it('should show confirmation modal when Clear All is clicked', async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      expect(screen.getByText('Clear All')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Clear All'));

    expect(screen.getByText('Clear All History')).toBeInTheDocument();
    expect(screen.getByText(/Are you sure you want to delete all transcriptions/i)).toBeInTheDocument();
  });

  it('should close confirmation modal when Cancel is clicked', async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      expect(screen.getByText('Clear All')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Clear All'));
    expect(screen.getByText('Clear All History')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Cancel'));

    await waitFor(() => {
      expect(screen.queryByText('Clear All History')).not.toBeInTheDocument();
    });
  });

  it('should clear all history when confirmed', async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      expect(screen.getByText('Clear All')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Clear All'));
    fireEvent.click(screen.getByText('Delete All'));

    expect(invoke).toHaveBeenCalledWith('clear_history');
  });

  it('should format duration correctly', async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      // Both entries have duration that rounds to 3s (2500ms and 3200ms)
      const durations = screen.getAllByText('3s');
      expect(durations.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('should show play button for entries with audio', async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      expect(screen.getByText('Hello world, this is a test transcription.')).toBeInTheDocument();
    });

    const playButtons = screen.getAllByTitle('Play audio');
    expect(playButtons.length).toBe(1); // Only first entry has audio_path
  });

  it('should render title correctly', async () => {
    render(<HistoryPanel />);

    await waitFor(() => {
      expect(screen.getByText('Transcription History')).toBeInTheDocument();
    });
  });
});
