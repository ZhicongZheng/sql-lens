import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  optimizeDeps: {
    include: ["monaco-editor"],
  },
  server: {
    port: 5174,
    proxy: {
      "/api": "http://127.0.0.1:5173",
      "/ws": {
        target: "ws://127.0.0.1:5173",
        ws: true,
      },
    },
  },
});
