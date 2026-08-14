// React Three Fiber imports the public `three` package namespace. Rey uses
// Three's modular WebGPU entry to keep the renderer graph inside the browser
// bundle budget, so expose that same graph plus the legacy renderer required
// by R3F's headless test harness. Both exports share Three's source modules and
// therefore the same constructor and color-management identities.
export * from "three/src/Three.WebGPU.js";
export { WebGLRenderer } from "three/src/renderers/WebGLRenderer.js";
