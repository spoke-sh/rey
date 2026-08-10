import stylex from "@stylexjs/unplugin";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

const fromUi = (path: string) => new URL(path, import.meta.url).pathname;

function stylexCssTarget(): Plugin {
  const resolvedId = "\0virtual:rey-stylex.css";

  return {
    name: "rey-stylex-css-target",
    resolveId(id) {
      return id === "virtual:rey-stylex.css" ? resolvedId : undefined;
    },
    load(id) {
      // A non-empty target makes Vite retain and link the asset before the
      // StyleX plugin appends its extracted atomic rules.
      return id === resolvedId ? "@layer rey-base;" : undefined;
    },
  };
}

export default defineConfig({
  build: {
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        assetFileNames: (asset) =>
          asset.names.some((name) => name.endsWith(".css"))
            ? "assets/app.css"
            : "assets/[name][extname]",
        entryFileNames: "assets/app.js",
      },
    },
  },
  plugins: [
    stylex.vite({ devMode: "full", useCSSLayers: true }),
    stylexCssTarget(),
    react(),
  ],
  resolve: {
    alias: [
      {
        find: "@hifi/kinetic/grammar",
        replacement: fromUi("./vendor/hifi/packages/kinetic/src/grammar.ts"),
      },
      {
        find: "@hifi/core",
        replacement: fromUi("./vendor/hifi/packages/core/src/index.ts"),
      },
      {
        find: "@hifi/kinetic",
        replacement: fromUi("./vendor/hifi/packages/kinetic/src/index.ts"),
      },
    ],
  },
});
