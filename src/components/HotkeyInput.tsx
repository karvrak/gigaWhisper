import { useState, useCallback } from 'react';
import { Keyboard } from 'lucide-react';

interface HotkeyInputProps {
  value: string;
  onChange: (value: string) => void;
}

export function HotkeyInput({ value, onChange }: HotkeyInputProps) {
  const [isRecording, setIsRecording] = useState(false);

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

  return (
    <div className="flex gap-2">
      <div
        className={`flex-1 px-3 py-2 border rounded-md flex items-center gap-2 ${
          isRecording
            ? 'border-blue-500 ring-2 ring-blue-200 dark:ring-blue-800'
            : 'border-gray-300 dark:border-gray-600'
        } bg-white dark:bg-gray-700`}
        tabIndex={0}
        onKeyDown={handleKeyDown}
        onFocus={() => setIsRecording(true)}
        onBlur={() => setIsRecording(false)}
      >
        <Keyboard className="w-4 h-4 text-gray-400" />
        <span className={isRecording ? 'text-blue-600' : ''}>
          {isRecording ? 'Press shortcut...' : value}
        </span>
      </div>
      <button
        onClick={() => setIsRecording(true)}
        className="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-700 text-sm"
      >
        Change
      </button>
    </div>
  );
}
