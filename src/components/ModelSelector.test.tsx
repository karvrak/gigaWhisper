import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ModelSelector } from './ModelSelector';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

vi.mock('@tauri-apps/api/core');
vi.mock('@tauri-apps/api/event');

const mockModels = [
  { model: 'tiny', path: '/models/tiny.bin', size_bytes: 78643200, downloaded: true },  // ~75 MB
  { model: 'base', path: '/models/base.bin', size_bytes: 141557760, downloaded: true }, // ~135 MB
  { model: 'small', path: '/models/small.bin', size_bytes: 488636416, downloaded: false }, // ~466 MB
  { model: 'medium', path: '/models/medium.bin', size_bytes: 1533018112, downloaded: false }, // ~1.4 GB
  { model: 'large', path: '/models/large.bin', size_bytes: 3086626816, downloaded: false }, // ~2.9 GB
];

describe('ModelSelector', () => {
  const mockOnChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_models') {
        return Promise.resolve(mockModels);
      }
      return Promise.resolve(undefined);
    });

    vi.mocked(listen).mockImplementation(() => {
      return Promise.resolve(() => {});
    });
  });

  it('should render model list', async () => {
    render(<ModelSelector value="base" onChange={mockOnChange} />);

    await waitFor(() => {
      expect(screen.getByText(/tiny/i)).toBeInTheDocument();
      expect(screen.getByText(/base/i)).toBeInTheDocument();
      expect(screen.getByText(/small/i)).toBeInTheDocument();
    });
  });

  it('should show download button for non-downloaded models', async () => {
    render(<ModelSelector value="base" onChange={mockOnChange} />);

    await waitFor(() => {
      const downloadButtons = screen.getAllByText('Download');
      expect(downloadButtons.length).toBeGreaterThan(0);
    });
  });

  it('should show Ready status for downloaded models', async () => {
    render(<ModelSelector value="base" onChange={mockOnChange} />);

    await waitFor(() => {
      const readyStatuses = screen.getAllByText('Ready');
      expect(readyStatuses.length).toBe(2); // tiny and base are downloaded
    });
  });

  it('should call onChange when a downloaded model is clicked', async () => {
    render(<ModelSelector value="base" onChange={mockOnChange} />);

    await waitFor(() => {
      expect(screen.getByText(/tiny/i)).toBeInTheDocument();
    });

    const tinyModel = screen.getByText(/tiny/i).closest('div[role="radio"]');
    fireEvent.click(tinyModel!);

    expect(mockOnChange).toHaveBeenCalledWith('tiny');
  });

  it('should not call onChange when a non-downloaded model is clicked', async () => {
    render(<ModelSelector value="base" onChange={mockOnChange} />);

    await waitFor(() => {
      expect(screen.getByText(/small/i)).toBeInTheDocument();
    });

    const smallModel = screen.getByText(/small/i).closest('div[role="radio"]');
    fireEvent.click(smallModel!);

    expect(mockOnChange).not.toHaveBeenCalled();
  });

  it('should start download when download button is clicked', async () => {
    render(<ModelSelector value="base" onChange={mockOnChange} />);

    await waitFor(() => {
      expect(screen.getByText(/small/i)).toBeInTheDocument();
    });

    const downloadButtons = screen.getAllByText('Download');
    fireEvent.click(downloadButtons[0]);

    expect(invoke).toHaveBeenCalledWith('download_model', { model: 'small' });
  });

  it('should format bytes correctly', async () => {
    render(<ModelSelector value="base" onChange={mockOnChange} />);

    await waitFor(() => {
      expect(screen.getByText(/75 MB/i)).toBeInTheDocument(); // tiny
      expect(screen.getByText(/135 MB/i)).toBeInTheDocument(); // base
    });
  });

  it('should display model descriptions', async () => {
    render(<ModelSelector value="base" onChange={mockOnChange} />);

    await waitFor(() => {
      expect(screen.getByText(/Fastest, lower accuracy/i)).toBeInTheDocument();
      expect(screen.getByText(/Good balance of speed and accuracy/i)).toBeInTheDocument();
    });
  });

  it('should have proper ARIA attributes', async () => {
    render(<ModelSelector value="base" onChange={mockOnChange} />);

    await waitFor(() => {
      const radioGroup = screen.getByRole('radiogroup');
      expect(radioGroup).toBeInTheDocument();

      const radios = screen.getAllByRole('radio');
      expect(radios.length).toBe(5);
    });
  });

  it('should mark selected model as checked', async () => {
    render(<ModelSelector value="base" onChange={mockOnChange} />);

    await waitFor(() => {
      const baseRadio = screen.getAllByRole('radio').find((el) =>
        el.textContent?.toLowerCase().includes('base')
      );
      expect(baseRadio).toHaveAttribute('aria-checked', 'true');
    });
  });
});
