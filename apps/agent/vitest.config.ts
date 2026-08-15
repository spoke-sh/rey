import stylex from "@stylexjs/unplugin";
import { defineConfig } from "vitest/config";

const THREE_WEBGPU_RUNTIME = new URL(
  "../../packages/explorer/src/three-fiber-runtime.ts",
  import.meta.url,
).pathname;
const REACT_THREE_FIBER_ESM = new URL(
  "../../packages/explorer/node_modules/@react-three/fiber/dist/react-three-fiber.esm.js",
  import.meta.url,
).pathname;

const TEST_ALIASES = [
  { find: /^three$/, replacement: THREE_WEBGPU_RUNTIME },
  { find: /^@react-three\/fiber$/, replacement: REACT_THREE_FIBER_ESM },
];

export default defineConfig({
  plugins: [stylex.rollup({ devMode: "full", useCSSLayers: true })],
  resolve: {
    alias: TEST_ALIASES,
    // Exercise the same ESM dependency graph that ships to the browser. The
    // CommonJS R3F entry loads `three` with Node's native require before Vite
    // can apply the exact-runtime alias above.
    mainFields: ["module", "main"],
  },
  test: {
    alias: TEST_ALIASES,
    include: ["src/**/*.test.{ts,tsx}", "scripts/**/*.test.mjs"],
    server: {
      deps: {
        inline: true,
      },
    },
  },
});
