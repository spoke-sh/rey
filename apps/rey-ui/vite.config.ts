import stylex from "@stylexjs/unplugin";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

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
        chunkFileNames: "assets/[name].js",
      },
    },
  },
  plugins: [
    stylex.vite({ devMode: "full", useCSSLayers: true }),
    stylexCssTarget(),
    react(),
  ],
});
