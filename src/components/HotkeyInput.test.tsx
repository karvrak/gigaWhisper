import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { HotkeyInput } from './HotkeyInput';

describe('HotkeyInput', () => {
  const mockOnChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render with initial value', () => {
    render(<HotkeyInput value="Ctrl+Space" onChange={mockOnChange} />);
    expect(screen.getByText('Ctrl+Space')).toBeInTheDocument();
  });

  it('should show "Press shortcut..." when focused', () => {
    render(<HotkeyInput value="Ctrl+Space" onChange={mockOnChange} />);
    const inputDiv = screen.getByText('Ctrl+Space').closest('div[tabindex="0"]');
    fireEvent.focus(inputDiv!);
    expect(screen.getByText('Press shortcut...')).toBeInTheDocument();
  });

  it('should capture Ctrl+A shortcut', () => {
    render(<HotkeyInput value="Ctrl+Space" onChange={mockOnChange} />);
    const inputDiv = screen.getByText('Ctrl+Space').closest('div[tabindex="0"]');
    fireEvent.focus(inputDiv!);
    fireEvent.keyDown(inputDiv!, {
      key: 'a',
      ctrlKey: true,
      altKey: false,
      shiftKey: false,
      metaKey: false,
    });
    expect(mockOnChange).toHaveBeenCalledWith('Ctrl+A');
  });

  it('should capture Ctrl+Shift+Space shortcut', () => {
    render(<HotkeyInput value="Ctrl+Space" onChange={mockOnChange} />);
    const inputDiv = screen.getByText('Ctrl+Space').closest('div[tabindex="0"]');
    fireEvent.focus(inputDiv!);
    fireEvent.keyDown(inputDiv!, {
      key: ' ',
      ctrlKey: true,
      altKey: false,
      shiftKey: true,
      metaKey: false,
    });
    expect(mockOnChange).toHaveBeenCalledWith('Ctrl+Shift+Space');
  });

  it('should not trigger onChange when only modifier keys are pressed', () => {
    render(<HotkeyInput value="Ctrl+Space" onChange={mockOnChange} />);
    const inputDiv = screen.getByText('Ctrl+Space').closest('div[tabindex="0"]');
    fireEvent.focus(inputDiv!);
    fireEvent.keyDown(inputDiv!, {
      key: 'Control',
      ctrlKey: true,
    });
    expect(mockOnChange).not.toHaveBeenCalled();
  });

  it('should start recording when Change button is clicked', () => {
    render(<HotkeyInput value="Ctrl+Space" onChange={mockOnChange} />);
    const changeButton = screen.getByRole('button', { name: /change/i });
    fireEvent.click(changeButton);
    expect(screen.getByText('Press shortcut...')).toBeInTheDocument();
  });

  it('should stop recording on blur', () => {
    render(<HotkeyInput value="Ctrl+Space" onChange={mockOnChange} />);
    const inputDiv = screen.getByText('Ctrl+Space').closest('div[tabindex="0"]');
    fireEvent.focus(inputDiv!);
    expect(screen.getByText('Press shortcut...')).toBeInTheDocument();
    fireEvent.blur(inputDiv!);
    expect(screen.getByText('Ctrl+Space')).toBeInTheDocument();
  });
});
