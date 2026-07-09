/**
 * Monaco Editor Vite configuration.
 *
 * Imports the base editor worker via Vite's `?worker` suffix so Monaco
 * runs fully locally (no CDN). Only the base worker is needed — SQL is a
 * built-in Monaco language and doesn't require a dedicated worker.
 *
 * This module must be imported before any `@monaco-editor/react` usage.
 */
import { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";

self.MonacoEnvironment = {
  getWorker() {
    return new editorWorker();
  },
};

loader.config({ monaco });
