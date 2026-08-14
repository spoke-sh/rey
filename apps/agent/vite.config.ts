import stylex from "@stylexjs/unplugin";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

const MAX_JAVASCRIPT_CHUNK_BYTES = 450 * 1024;
const THREE_WEBGPU_RUNTIME = new URL(
  "../../packages/explorer/src/three-fiber-runtime.ts",
  import.meta.url,
).pathname;

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

function javascriptChunkBudget(): Plugin {
  return {
    name: "rey-javascript-chunk-budget",
    generateBundle: {
      order: "post",
      handler(_options, bundle) {
        const chunks = Object.values(bundle)
          .filter((output) => output.type === "chunk")
          .map((chunk) => ({
            bytes: new TextEncoder().encode(chunk.code).byteLength,
            dynamic_entry: chunk.isDynamicEntry,
            dynamic_imports: [...chunk.dynamicImports].sort(),
            entry: chunk.isEntry,
            file: chunk.fileName,
            imports: [...chunk.imports].sort(),
          }))
          .sort((left, right) => left.file.localeCompare(right.file));
        const oversized = chunks.filter(
          (chunk) => chunk.bytes > MAX_JAVASCRIPT_CHUNK_BYTES,
        );
        if (oversized.length > 0) {
          this.error(
            `JavaScript chunks exceed the ${MAX_JAVASCRIPT_CHUNK_BYTES}-byte bound: ${oversized
              .map((chunk) => `${chunk.file} (${chunk.bytes})`)
              .join(", ")}`,
          );
        }
        this.emitFile({
          type: "asset",
          fileName: "bundle-report.json",
          source: `${JSON.stringify(
            {
              schema: "rey.ui-bundle-report.v1",
              bundler: "vite@8.2.1+rolldown@1.2.3",
              limits: {
                max_javascript_chunk_bytes: MAX_JAVASCRIPT_CHUNK_BYTES,
              },
              chunks,
            },
            null,
            2,
          )}\n`,
        });
      },
    },
  };
}

export default defineConfig({
  base: "/",
  resolve: {
    // R3F imports `three` internally. Bind that exact import to the same
    // modular WebGPU runtime exported by @rey/explorer so constructor and
    // color-management identities cannot diverge across the package boundary.
    alias: [{ find: /^three$/, replacement: THREE_WEBGPU_RUNTIME }],
  },
  build: {
    cssCodeSplit: false,
    rolldownOptions: {
      output: {
        // Three's WebGPU/TSL graph contains module cycles. Preserve source
        // execution order when the bounded vendor groups are split so a
        // subclass is never evaluated before its base class is initialized.
        strictExecutionOrder: true,
        assetFileNames: (asset) =>
          asset.names.some((name) => name.endsWith(".css"))
            ? "assets/app.css"
            : "assets/[name][extname]",
        entryFileNames: "assets/app.js",
        chunkFileNames: "assets/[name].js",
        codeSplitting: {
          minSize: 20 * 1024,
          maxSize: 450 * 1024,
          groups: [
            {
              name: "react",
              test: /node_modules[\\/](?:react|react-dom|scheduler)[\\/]/,
              priority: 30,
            },
            {
              name: "tanstack-router",
              test: /node_modules[\\/]@tanstack[\\/](?:react-router|router-core)[\\/]/,
              priority: 20,
            },
            {
              name: "react-three-fiber",
              test: /node_modules[\\/]@react-three[\\/]fiber[\\/]/,
              priority: 15,
              maxSize: 400 * 1024,
            },
            {
              name: "three",
              test: /node_modules[\\/]three[\\/]/,
              priority: 10,
              maxSize: 400 * 1024,
            },
          ],
        },
      },
    },
  },
  plugins: [
    stylex.vite({ devMode: "full", useCSSLayers: true }),
    stylexCssTarget(),
    react(),
    javascriptChunkBudget(),
  ],
});
