import { useState, useCallback, useRef } from 'react';
import { Keyboard } from 'lucide-react';

interface HotkeyInputProps {
  value: string;
  onChange: (value: string) => void;
}

const MOUSE_BUTTON_NAMES: Record<number, string> = {
  3: 'Mouse4',
  4: 'Mouse5',
};

export function HotkeyInput({ value, onChange }: HotkeyInputProps) {
  const [isRecording, setIsRecording] = useState(false);
  const inputRef = useRef<HTMLDivElement>(null);

  const startRecording = useCallback(() => {
    setIsRecording(true);
    // Focus the input div so it captures keyboard events
    inputRef.current?.focus();
  }, []);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (!isRecording) return;

      e.preventDefault();
      e.stopPropagation();

      const parts: string[] = [];

      if (e.ctrlKey) parts.push('Ctrl');
      if (e.altKey) parts.push('Alt');
      if (e.shiftKey) parts.push('Shift');
      if (e.metaKey) parts.push('Super');

      // Get the key name
      const key = e.key;
      if (!['Control', 'Alt', 'Shift', 'Meta'].includes(key)) {
        // Normalize key names
        const normalizedKey =
          key === ' '
            ? 'Space'
            : key.length === 1
            ? key.toUpperCase()
            : key;

        parts.push(normalizedKey);
        onChange(parts.join('+'));
        setIsRecording(false);
      }
    },
    [isRecording, onChange]
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (!isRecording) return;

      // Only capture extra mouse buttons (Mouse4, Mouse5, etc.)
      // Buttons 0 (left), 1 (middle), 2 (right) are standard navigation buttons
      const buttonName = MOUSE_BUTTON_NAMES[e.button];
      if (!buttonName) return;

      e.preventDefault();
      e.stopPropagation();

      const parts: string[] = [];

      if (e.ctrlKey) parts.push('Ctrl');
      if (e.altKey) parts.push('Alt');
      if (e.shiftKey) parts.push('Shift');
      if (e.metaKey) parts.push('Super');

      parts.push(buttonName);
      onChange(parts.join('+'));
      setIsRecording(false);
    },
    [isRecording, onChange]
  );

  return (
    <div className="flex gap-2">
      <div
        ref={inputRef}
        className={`flex-1 px-3 py-2 border rounded-md flex items-center gap-2 cursor-pointer ${
          isRecording
            ? 'border-accent ring-2 ring-accent/30'
            : 'border-edge'
        } bg-surface-tertiary`}
        tabIndex={0}
        onKeyDown={handleKeyDown}
        onMouseDown={handleMouseDown}
        onFocus={() => setIsRecording(true)}
        onBlur={() => setIsRecording(false)}
      >
        <Keyboard className="w-4 h-4 text-content-tertiary" />
        <span className={isRecording ? 'text-accent' : ''}>
          {isRecording ? 'Press shortcut or mouse button...' : value}
        </span>
      </div>
      <button
        onMouseDown={(e) => {
          e.preventDefault(); // Prevent stealing focus from the input
          startRecording();
        }}
        className="px-3 py-2 border border-edge rounded-md hover:bg-surface-tertiary text-sm"
      >
        Change
      </button>
    </div>
  );
}
