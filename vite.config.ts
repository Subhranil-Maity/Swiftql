import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

const host = process.env.TAURI_DEV_HOST;

/**
 * libpg-query (SQL syntax checking in the console) loads `libpg-query.wasm`
 * relative to the page, i.e. from `/libpg-query.wasm`, and the bundler can't
 * see that reference. Serve the file there in dev and emit it there in builds.
 */
function libpgQueryWasm(): Plugin {
  const file = path.resolve(import.meta.dirname, "node_modules/libpg-query/wasm/libpg-query.wasm");
  return {
    name: "libpg-query-wasm",
    configureServer(server) {
      server.middlewares.use("/libpg-query.wasm", (_req, res) => {
        res.setHeader("Content-Type", "application/wasm");
        fs.createReadStream(file).pipe(res);
      });
    },
    generateBundle() {
      this.emitFile({ type: "asset", fileName: "libpg-query.wasm", source: fs.readFileSync(file) });
    },
  };
}

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react(), tailwindcss(), libpgQueryWasm()],
  resolve: {
    alias: {
      "@": path.resolve(import.meta.dirname, "./src"),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
