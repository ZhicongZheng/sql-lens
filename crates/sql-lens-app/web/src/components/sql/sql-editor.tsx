import { useEffect, useMemo } from "react";
import Editor, { useMonaco } from "@monaco-editor/react";

import { useTheme } from "@/app/providers/theme-provider";

// Ensure Monaco config is loaded (worker setup).
import "@/lib/monaco-config";

interface SqlEditorProps {
  value: string;
  language?: string;
  height?: string;
}

/**
 * Read-only Monaco Editor for SQL display.
 * Follows the app's light/dark theme. Auto-sizes height to content (capped).
 */
export function SqlEditor({
  value,
  language = "sql",
  height,
}: SqlEditorProps) {
  const { theme } = useTheme();
  const monacoTheme = theme === "dark" ? "vs-dark" : "vs";

  // Auto-calculate height based on line count (capped at 400px).
  const computedHeight = useMemo(() => {
    if (height) return height;
    const lines = value.split("\n").length;
    const px = Math.max(80, Math.min(lines * 20, 400));
    return `${px}px`;
  }, [value, height]);

  // Sync theme when it changes without a full re-mount.
  const monaco = useMonaco();
  useEffect(() => {
    if (monaco) {
      monaco.editor.setTheme(monacoTheme);
    }
  }, [monaco, monacoTheme]);

  return (
    <Editor
      height={computedHeight}
      language={language}
      value={value}
      theme={monacoTheme}
      loading={
        <div className="flex h-20 items-center justify-center text-xs text-muted-foreground">
          Loading editor…
        </div>
      }
      options={{
        readOnly: true,
        domReadOnly: true,
        minimap: { enabled: false },
        wordWrap: "on",
        lineNumbers: "on",
        lineNumbersMinChars: 3,
        scrollBeyondLastLine: false,
        automaticLayout: true,
        cursorBlinking: "solid",
        fontFamily: "ui-monospace, monospace",
        fontSize: 13,
        padding: { top: 8, bottom: 8 },
        renderLineHighlight: "none",
        overviewRulerLanes: 0,
        hideCursorInOverviewRuler: true,
        scrollbar: {
          vertical: "auto",
          horizontal: "auto",
          verticalScrollbarSize: 8,
          horizontalScrollbarSize: 8,
        },
      }}
    />
  );
}
