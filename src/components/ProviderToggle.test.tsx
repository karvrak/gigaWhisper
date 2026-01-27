import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ProviderToggle } from './ProviderToggle';

describe('ProviderToggle', () => {
  const mockOnChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render both provider options', () => {
    render(<ProviderToggle value="local" onChange={mockOnChange} />);

    expect(screen.getByText('Local')).toBeInTheDocument();
    expect(screen.getByText('Groq Cloud')).toBeInTheDocument();
  });

  it('should highlight local option when selected', () => {
    render(<ProviderToggle value="local" onChange={mockOnChange} />);

    const localButton = screen.getByText('Local').closest('button');
    expect(localButton).toHaveClass('border-blue-500');
  });

  it('should highlight groq option when selected', () => {
    render(<ProviderToggle value="groq" onChange={mockOnChange} />);

    const groqButton = screen.getByText('Groq Cloud').closest('button');
    expect(groqButton).toHaveClass('border-blue-500');
  });

  it('should call onChange with "local" when local is clicked', () => {
    render(<ProviderToggle value="groq" onChange={mockOnChange} />);

    const localButton = screen.getByText('Local').closest('button');
    fireEvent.click(localButton!);

    expect(mockOnChange).toHaveBeenCalledWith('local');
  });

  it('should call onChange with "groq" when groq is clicked', () => {
    render(<ProviderToggle value="local" onChange={mockOnChange} />);

    const groqButton = screen.getByText('Groq Cloud').closest('button');
    fireEvent.click(groqButton!);

    expect(mockOnChange).toHaveBeenCalledWith('groq');
  });

  it('should display descriptions for each provider', () => {
    render(<ProviderToggle value="local" onChange={mockOnChange} />);

    expect(screen.getByText(/Run whisper.cpp on your machine/i)).toBeInTheDocument();
    expect(screen.getByText(/Fast cloud transcription/i)).toBeInTheDocument();
  });
});
