import Editor from "@monaco-editor/react";
import { useTheme } from "@/app/providers/theme-provider";

interface SqlMonacoViewerProps {
  sql: string;
  height?: string;
}

export function SqlMonacoViewer({ sql, height = "200px" }: SqlMonacoViewerProps) {
  const { theme } = useTheme();
  const monacoTheme = theme === "dark" ? "vs-dark" : "vs";

  const handleCopy = async () => {
    await navigator.clipboard.writeText(sql);
  };

  return (
    <div className="relative rounded-md border">
      <button
        onClick={handleCopy}
        className="absolute right-2 top-2 z-10 rounded bg-background/80 px-2 py-1 text-xs hover:bg-background"
        aria-label="Copy SQL to clipboard"
      >
        Copy
      </button>
      <Editor
        height={height}
        defaultLanguage="sql"
        value={sql}
        theme={monacoTheme}
        options={{
          readOnly: true,
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
          fontSize: 12,
          lineNumbers: "off",
          folding: false,
          renderLineHighlight: "none",
        }}
      />
    </div>
  );
}
