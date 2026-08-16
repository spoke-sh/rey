#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { cpus, hostname, platform, release, tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  evaluateLandscapeCapture,
  landscapeWorkload,
  validateLandscapeWorkloadSuite,
} from "./explorer-landscape-qualification.mjs";

const SCHEMA = "rey.explorer-qualification-voyage.v1";
const AUTHORITY =
  "retained local browser qualification only; not semantic evidence, GPU execution timing, frame-rate proof, action authority, or proof authority";
const STAGES = ["world", "atlas", "landscape", "objects", "evidence"];
const CAPTURE_ORDER = [
  "world",
  "backend-loss",
  "atlas",
  "landscape",
  "objects",
  "evidence",
  "passive-revalidation",
];
const REPOSITORY_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
const DIST_ROOT = join(REPOSITORY_ROOT, "apps/agent/dist");
const LANDSCAPE_WORKLOAD_SUITE = join(
  REPOSITORY_ROOT,
  "apps/agent/qualification/explorer-landscape-workloads.json",
);

function usage() {
  return `Retain one bounded World → Atlas → County → Evidence browser voyage.

Usage:
  pnpm qualify:explorer -- --base-url URL --backend BACKEND [options]

Required:
  --base-url URL        Running rey agent origin, for example http://127.0.0.1:5715
  --backend BACKEND     reference, webgl2, or webgpu

Options:
  --browser PATH        Chrome/Chromium executable (auto-detected by default)
  --width PIXELS        Viewport width (default: 1920)
  --height PIXELS       Viewport height (default: 1080)
  --dpr NUMBER          Device pixel ratio (default: 1)
  --region ID           Admitted scene region to enter (default: rey-county)
  --landscape-workload ID
                        Bind and assert one named Landscape fidelity workload
  --loss MODE           none (default), webgl-context, or webgpu-device
  --revalidation MODE   none (default) or attention
  --output-dir PATH     Retained output root (default: .rey/qualification/explorer)
  --transport MODE      direct (default) or fulfilled for socket-restricted Chrome
  --timeout-ms NUMBER   Per-stage timeout (default: 30000)
  --help                Show this help

Generated PNG and JSON evidence stays beneath ignored .rey state by default.`;
}

function parseArguments(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--") continue;
    if (token === "--help" || token === "-h") return { help: true };
    if (!token?.startsWith("--"))
      throw new Error(`unexpected argument ${token}`);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`${token} requires a value`);
    }
    values.set(token.slice(2), value);
    index += 1;
  }

  const baseUrl = values.get("base-url");
  const backend = values.get("backend");
  if (!baseUrl) throw new Error("--base-url is required");
  if (!new Set(["reference", "webgl2", "webgpu"]).has(backend)) {
    throw new Error("--backend must be reference, webgl2, or webgpu");
  }
  const transport = values.get("transport") ?? "direct";
  if (!new Set(["direct", "fulfilled"]).has(transport)) {
    throw new Error("--transport must be direct or fulfilled");
  }
  const loss = values.get("loss") ?? "none";
  if (!new Set(["none", "webgl-context", "webgpu-device"]).has(loss)) {
    throw new Error("--loss must be none, webgl-context, or webgpu-device");
  }
  if (
    (loss === "webgl-context" && backend !== "webgl2") ||
    (loss === "webgpu-device" && backend !== "webgpu")
  ) {
    throw new Error("--loss must match the requested accelerated backend");
  }
  const revalidation = values.get("revalidation") ?? "none";
  if (!new Set(["none", "attention"]).has(revalidation)) {
    throw new Error("--revalidation must be none or attention");
  }
  if (revalidation !== "none" && transport !== "fulfilled") {
    throw new Error(
      "--revalidation attention is a fulfilled-transport qualification stimulus",
    );
  }
  let origin;
  try {
    origin = new URL(baseUrl).origin;
  } catch {
    throw new Error("--base-url must be an absolute HTTP URL");
  }
  if (!/^https?:$/.test(new URL(origin).protocol)) {
    throw new Error("--base-url must use HTTP or HTTPS");
  }

  const positiveNumber = (name, fallback, integer = true) => {
    const raw = values.get(name);
    if (raw === undefined) return fallback;
    const number = Number(raw);
    if (
      !Number.isFinite(number) ||
      number <= 0 ||
      (integer && !Number.isInteger(number))
    ) {
      throw new Error(
        `--${name} must be a positive ${integer ? "integer" : "number"}`,
      );
    }
    return number;
  };

  return {
    backend,
    baseUrl: origin,
    browser: values.get("browser"),
    devicePixelRatio: positiveNumber("dpr", 1, false),
    height: positiveNumber("height", 1080),
    landscapeWorkload: values.get("landscape-workload") ?? null,
    loss,
    outputRoot: resolve(
      values.get("output-dir") ??
        join(REPOSITORY_ROOT, ".rey/qualification/explorer"),
    ),
    region: values.get("region") ?? "rey-county",
    revalidation,
    timeoutMs: positiveNumber("timeout-ms", 30_000),
    transport,
    width: positiveNumber("width", 1920),
  };
}

function findBrowser(explicit) {
  if (explicit) return explicit;
  for (const candidate of [
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
  ]) {
    const found = spawnSync("sh", ["-c", `command -v ${candidate}`], {
      encoding: "utf8",
    });
    if (found.status === 0 && found.stdout.trim()) return found.stdout.trim();
  }
  throw new Error("Chrome or Chromium was not found; pass --browser PATH");
}

function timestamp() {
  return new Date().toISOString().replaceAll(":", "-").replaceAll(".", "-");
}

function sha256(buffer) {
  return `sha256:${createHash("sha256").update(buffer).digest("hex")}`;
}

function expectedLossConsoleEntry(entry, loss) {
  return (
    loss === "webgl-context" &&
    entry.level === "error" &&
    entry.source === "console-api" &&
    entry.text?.startsWith("THREE.THREE.WebGPURenderer: WebGL Device Lost:")
  );
}

function sleep(milliseconds) {
  return new Promise((resolvePromise) =>
    setTimeout(resolvePromise, milliseconds),
  );
}

class CdpConnection {
  #id = 0;
  #listeners = new Map();
  #pending = new Map();
  #socket;

  static async connect(url) {
    const socket = new WebSocket(url);
    await new Promise((resolvePromise, reject) => {
      socket.addEventListener("open", resolvePromise, { once: true });
      socket.addEventListener(
        "error",
        () => reject(new Error("Chrome DevTools websocket failed to open")),
        { once: true },
      );
    });
    return new CdpConnection(socket);
  }

  constructor(socket) {
    this.#socket = socket;
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(event.data);
      if (message.id) {
        const pending = this.#pending.get(message.id);
        if (!pending) return;
        this.#pending.delete(message.id);
        clearTimeout(pending.timer);
        if (message.error) pending.reject(new Error(message.error.message));
        else pending.resolve(message.result);
        return;
      }
      for (const listener of this.#listeners.get(message.method) ?? []) {
        listener(message.params ?? {});
      }
    });
  }

  on(method, listener) {
    const listeners = this.#listeners.get(method) ?? [];
    listeners.push(listener);
    this.#listeners.set(method, listeners);
  }

  send(method, params = {}) {
    const id = ++this.#id;
    return new Promise((resolvePromise, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(new Error(`Chrome DevTools command timed out: ${method}`));
      }, 30_000);
      this.#pending.set(id, { reject, resolve: resolvePromise, timer });
      this.#socket.send(JSON.stringify({ id, method, params }));
    });
  }

  async evaluate(expression) {
    const response = await this.send("Runtime.evaluate", {
      awaitPromise: true,
      expression,
      returnByValue: true,
    });
    if (response.exceptionDetails) {
      throw new Error(
        response.exceptionDetails.exception?.description ??
          response.exceptionDetails.text ??
          "browser evaluation failed",
      );
    }
    return response.result.value;
  }

  close() {
    this.#socket.close();
  }
}

async function launchChrome(browser, backend, route, fulfilledDocuments) {
  const profile = await mkdtemp(join(tmpdir(), "rey-explorer-qualification-"));
  let initialUrl = "about:blank";
  let bootstrapPath = null;
  if (fulfilledDocuments) {
    bootstrapPath = join(profile, "explore.html");
    const serializedDocuments = JSON.stringify(fulfilledDocuments).replaceAll(
      "<",
      "\\u003c",
    );
    const distributionRoot = `${pathToFileURL(DIST_ROOT).href}/`;
    const bootstrap = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Rey / Explorer qualification</title>
    <link rel="stylesheet" href="${pathToFileURL(join(DIST_ROOT, "assets/app.css")).href}" />
    <script>
      const documents = ${serializedDocuments};
      const distributionRoot = ${JSON.stringify(distributionRoot)};
      // A module worker cannot load from this file-origin bootstrap. Exercise
      // the engine's bounded fallback; direct transport owns worker coverage.
      globalThis.Worker = undefined;
      const appendChild = Node.prototype.appendChild;
      Node.prototype.appendChild = function (node) {
        if (
          node instanceof HTMLLinkElement &&
          node.rel === "modulepreload" &&
          node.href.startsWith("file:///assets/")
        ) {
          node.href = new URL(node.href.slice("file:///".length), distributionRoot).href;
        }
        return appendChild.call(this, node);
      };
      globalThis.__reyQualificationFetchCounts = {};
      globalThis.fetch = async (input) => {
        const value = typeof input === "string" ? input : input.url;
        const pathname = value.startsWith("/")
          ? new URL(value, "http://rey.qualification").pathname
          : new URL(value).pathname;
        const count = (globalThis.__reyQualificationFetchCounts[pathname] ?? 0) + 1;
        globalThis.__reyQualificationFetchCounts[pathname] = count;
        const retained = documents[pathname];
        const document = retained?.rey_qualification_sequence
          ? retained.responses[Math.min(count - 1, retained.responses.length - 1)]
          : retained;
        if (document === undefined) {
          return new Response("qualification bootstrap has no retained response", { status: 404 });
        }
        return new Response(JSON.stringify(document), {
          headers: { "Content-Type": "application/json; charset=utf-8" },
          status: 200,
        });
      };
    </script>
    <script type="module" src="${pathToFileURL(join(DIST_ROOT, "assets/app.js")).href}"></script>
  </head>
  <body><div id="root"></div></body>
</html>`;
    await writeFile(bootstrapPath, bootstrap);
    initialUrl = `${pathToFileURL(bootstrapPath).href}${route.search}`;
  }
  const flags = [
    "--headless=new",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--disable-background-networking",
    "--disable-component-update",
    "--disable-default-apps",
    "--disable-extensions",
    "--disable-sync",
    "--metrics-recording-only",
    "--no-default-browser-check",
    "--no-first-run",
    "--remote-debugging-address=127.0.0.1",
    "--remote-debugging-port=0",
    `--user-data-dir=${profile}`,
  ];
  if (fulfilledDocuments) flags.push("--allow-file-access-from-files");
  if (backend === "webgl2") {
    flags.push(
      "--use-gl=angle",
      "--use-angle=swiftshader",
      "--enable-unsafe-swiftshader",
    );
  } else if (backend === "webgpu") {
    flags.push(
      "--enable-features=Vulkan",
      "--enable-unsafe-webgpu",
      "--use-angle=vulkan",
      "--use-vulkan=swiftshader",
    );
  }
  flags.push(initialUrl);

  const child = spawn(browser, flags, { stdio: ["ignore", "ignore", "pipe"] });
  let stderr = "";
  const devtoolsUrl = await new Promise((resolvePromise, reject) => {
    const timer = setTimeout(() => {
      reject(
        new Error(`Chrome did not expose DevTools: ${stderr.slice(-1_000)}`),
      );
    }, 15_000);
    child.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
      const match = /DevTools listening on (ws:\/\/[^\s]+)/.exec(stderr);
      if (!match) return;
      clearTimeout(timer);
      resolvePromise(match[1]);
    });
    child.once("exit", (code) => {
      clearTimeout(timer);
      reject(new Error(`Chrome exited before DevTools was ready (${code})`));
    });
  });
  const debugOrigin = devtoolsUrl
    .replace(/^ws:/, "http:")
    .replace(/\/devtools\/browser\/.*$/, "");
  let target;
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const targetResponse = await fetch(`${debugOrigin}/json/list`);
    if (!targetResponse.ok)
      throw new Error("Chrome could not list page targets");
    const targets = await targetResponse.json();
    target = targets.find(
      (candidate) =>
        candidate.type === "page" && candidate.webSocketDebuggerUrl,
    );
    if (target) break;
    await sleep(100);
  }
  if (!target) throw new Error("Chrome did not expose the qualification page");
  return {
    bootstrapPath,
    bootstrapUrl: initialUrl,
    child,
    connection: await CdpConnection.connect(target.webSocketDebuggerUrl),
    initialTargetUrl: target.url,
    profile,
    stderr: () => stderr,
  };
}

async function waitFor(connection, expression, description, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const value = await connection.evaluate(`Boolean(${expression})`);
      if (value) return value;
    } catch (error) {
      lastError = error;
    }
    await sleep(125);
  }
  throw new Error(
    `timed out waiting for ${description}${lastError ? `: ${lastError.message}` : ""}`,
  );
}

async function dispatchClick(
  connection,
  selectorExpression,
  description,
  timeoutMs,
) {
  await waitFor(connection, selectorExpression, description, timeoutMs);
  const clicked = await connection.evaluate(`(() => {
    const element = ${selectorExpression};
    if (!element) return false;
    element.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, view: window }));
    return true;
  })()`);
  if (!clicked) throw new Error(`could not activate ${description}`);
}

async function measureInteraction(interactions, name, operation) {
  const startedAt = performance.now();
  await operation();
  interactions.push({
    elapsed_ms: Number((performance.now() - startedAt).toFixed(3)),
    name,
  });
}

async function rotateGlobeToRegion(connection, longitudeMicrodegrees) {
  const viewport = await connection.evaluate(`(() => {
    const bounds = document.querySelector('[role="application"]')?.getBoundingClientRect();
    return bounds ? { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height } : null;
  })()`);
  if (!viewport) throw new Error("the Explorer globe viewport is unavailable");
  const start = {
    x: viewport.x + viewport.width / 2,
    y: viewport.y + viewport.height / 2,
  };
  const deltaX = -(longitudeMicrodegrees / 1_000_000) / 0.22;
  await connection.send("Input.dispatchMouseEvent", {
    button: "left",
    buttons: 1,
    clickCount: 1,
    type: "mousePressed",
    x: start.x,
    y: start.y,
  });
  for (let index = 1; index <= 8; index += 1) {
    await connection.send("Input.dispatchMouseEvent", {
      button: "left",
      buttons: 1,
      type: "mouseMoved",
      x: start.x + (deltaX * index) / 8,
      y: start.y,
    });
  }
  await connection.send("Input.dispatchMouseEvent", {
    button: "left",
    buttons: 0,
    clickCount: 1,
    type: "mouseReleased",
    x: start.x + deltaX,
    y: start.y,
  });
}

async function panOutsideGlobe(connection) {
  const before = await connection.evaluate(`(() => {
    const viewport = document.querySelector('[role="application"]');
    const atmosphere = document.querySelector('[data-globe-atmosphere]');
    const sphere = document.querySelector('[data-globe-sphere]');
    const bounds = viewport?.getBoundingClientRect();
    const renderedAtmosphereBounds = atmosphere?.getBoundingClientRect();
    const sphereBounds = sphere?.getBoundingClientRect();
    const haloScale = Number(sphere?.getAttribute('data-globe-halo-scale'));
    const atmosphereBounds = renderedAtmosphereBounds ?? (sphereBounds && Number.isFinite(haloScale) ? {
      x: sphereBounds.x - sphereBounds.width * (haloScale - 1) / 2,
      y: sphereBounds.y - sphereBounds.height * (haloScale - 1) / 2,
      width: sphereBounds.width * haloScale,
      height: sphereBounds.height * haloScale,
    } : null);
    return viewport && bounds && atmosphereBounds ? {
      atmosphere_bounds: {
        x: atmosphereBounds.x,
        y: atmosphereBounds.y,
        width: atmosphereBounds.width,
        height: atmosphereBounds.height,
      },
      bounds: { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height },
      pan_x: Number(viewport.getAttribute("data-camera-pan-x")),
      pan_y: Number(viewport.getAttribute("data-camera-pan-y")),
      pitch: Number(viewport.getAttribute("data-globe-pitch")),
      yaw: Number(viewport.getAttribute("data-globe-yaw")),
    } : null;
  })()`);
  if (!before)
    throw new Error("the Explorer globe camera state is unavailable");
  const outsideGap = before.atmosphere_bounds.x - before.bounds.x;
  const start = {
    x: before.bounds.x + Math.max(8, outsideGap / 2),
    y: before.atmosphere_bounds.y + before.atmosphere_bounds.height / 2,
  };
  const delta = { x: 84, y: 28 };
  await connection.send("Input.dispatchMouseEvent", {
    button: "left",
    buttons: 1,
    clickCount: 1,
    type: "mousePressed",
    x: start.x,
    y: start.y,
  });
  await connection.send("Input.dispatchMouseEvent", {
    button: "left",
    buttons: 1,
    type: "mouseMoved",
    x: start.x + delta.x,
    y: start.y + delta.y,
  });
  await connection.send("Input.dispatchMouseEvent", {
    button: "left",
    buttons: 0,
    clickCount: 1,
    type: "mouseReleased",
    x: start.x + delta.x,
    y: start.y + delta.y,
  });
  await sleep(50);
  const after = await connection.evaluate(`(() => {
    const viewport = document.querySelector('[role="application"]');
    return viewport ? {
      pan_x: Number(viewport.getAttribute("data-camera-pan-x")),
      pan_y: Number(viewport.getAttribute("data-camera-pan-y")),
      pitch: Number(viewport.getAttribute("data-globe-pitch")),
      yaw: Number(viewport.getAttribute("data-globe-yaw")),
    } : null;
  })()`);
  return {
    after,
    before: {
      pan_x: before.pan_x,
      pan_y: before.pan_y,
      pitch: before.pitch,
      yaw: before.yaw,
    },
    delta,
    observed:
      after !== null &&
      Math.abs(after.pan_x - (before.pan_x + delta.x)) < 0.001 &&
      Math.abs(after.pan_y - (before.pan_y + delta.y)) < 0.001 &&
      after.pitch === before.pitch &&
      after.yaw === before.yaw,
  };
}

async function verifySmoothWorldWheelZoom(connection, timeoutMs) {
  const stateExpression = `(() => {
    const viewport = document.querySelector('[role="application"]');
    const bounds = viewport?.getBoundingClientRect();
    return viewport && bounds ? {
      bounds: { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height },
      pan_x: Number(viewport.getAttribute("data-camera-pan-x")),
      pan_y: Number(viewport.getAttribute("data-camera-pan-y")),
      rendered_scale: Number(viewport.getAttribute("data-camera-rendered-scale")),
      zoom: Number(viewport.getAttribute("data-camera-zoom")),
    } : null;
  })()`;
  const before = await connection.evaluate(stateExpression);
  if (!before)
    throw new Error("the Explorer wheel camera state is unavailable");
  const pointer = {
    x: before.bounds.x + before.bounds.width * 0.72,
    y: before.bounds.y + before.bounds.height * 0.42,
  };
  const wheel = (deltaY) =>
    connection.send("Input.dispatchMouseEvent", {
      deltaX: 0,
      deltaY,
      type: "mouseWheel",
      x: pointer.x,
      y: pointer.y,
    });
  await wheel(-100);
  const animationFrames = await connection.evaluate(`new Promise((resolve) => {
    const samples = [];
    const sample = () => {
      const viewport = document.querySelector('[role="application"]');
      samples.push({
        rendered_scale: Number(viewport?.getAttribute("data-camera-rendered-scale")),
        zoom: Number(viewport?.getAttribute("data-camera-zoom")),
      });
      if (samples.length >= 10) resolve(samples);
      else requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  })`);
  await waitFor(
    connection,
    `Number(document.querySelector('[role="application"]')?.getAttribute("data-camera-zoom")) > ${before.zoom + 0.04}`,
    "smoothed World wheel step",
    timeoutMs,
  );
  const afterFirst = await connection.evaluate(stateExpression);
  await wheel(-3);
  await waitFor(
    connection,
    `Number(document.querySelector('[role="application"]')?.getAttribute("data-camera-zoom")) > ${afterFirst.zoom + 0.001}`,
    "small smoothed World wheel step",
    timeoutMs,
  );
  const afterSecond = await connection.evaluate(stateExpression);
  await dispatchClick(
    connection,
    `[...document.querySelectorAll("button")].find((button) => button.textContent?.trim() === "FIT")`,
    "Explorer fit control",
    timeoutMs,
  );
  await waitFor(
    connection,
    `Number(document.querySelector('[role="application"]')?.getAttribute("data-camera-zoom")) === 0.26`,
    "Explorer fit reset",
    timeoutMs,
  );
  await dispatchClick(
    connection,
    `document.querySelector('[aria-label="Zoom out one semantic level"]')`,
    "World zoom control",
    timeoutMs,
  );
  await waitFor(
    connection,
    `Number(document.querySelector('[role="application"]')?.getAttribute("data-camera-zoom")) === 0.1`,
    "World zoom reset",
    timeoutMs,
  );
  const pointerFromCenter = {
    x: pointer.x - before.bounds.x - before.bounds.width / 2,
    y: pointer.y - before.bounds.y - before.bounds.height / 2,
  };
  const anchoredCoordinate = (state) => ({
    x: (pointerFromCenter.x - state.pan_x) / state.rendered_scale,
    y: (pointerFromCenter.y - state.pan_y) / state.rendered_scale,
  });
  const beforeAnchor = anchoredCoordinate(before);
  const firstAnchor = anchoredCoordinate(afterFirst);
  const secondAnchor = anchoredCoordinate(afterSecond);
  const distinctZoomFrames = new Set(
    animationFrames.map(({ zoom }) => zoom.toFixed(6)),
  ).size;
  const monotonicFrames = animationFrames.every(
    ({ zoom }, index) => index === 0 || zoom >= animationFrames[index - 1].zoom,
  );
  return {
    animation_frames: animationFrames,
    after_first: afterFirst,
    after_second: afterSecond,
    before,
    distinct_zoom_frames: distinctZoomFrames,
    observed:
      afterFirst !== null &&
      afterSecond !== null &&
      distinctZoomFrames >= 4 &&
      monotonicFrames &&
      afterSecond.zoom > afterFirst.zoom &&
      afterFirst.rendered_scale !== before.rendered_scale &&
      afterSecond.rendered_scale !== afterFirst.rendered_scale &&
      Math.abs(firstAnchor.x - beforeAnchor.x) < 0.25 &&
      Math.abs(firstAnchor.y - beforeAnchor.y) < 0.25 &&
      Math.abs(secondAnchor.x - beforeAnchor.x) < 0.25 &&
      Math.abs(secondAnchor.y - beforeAnchor.y) < 0.25,
  };
}

async function verifyRotatedWorldAtlasUnfurl(connection, timeoutMs) {
  const before = await connection.evaluate(`(() => {
    const viewport = document.querySelector('[role="application"]');
    const bounds = viewport?.getBoundingClientRect();
    return viewport && bounds ? {
      bounds: { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height },
      pitch: Number(viewport.getAttribute("data-globe-pitch")),
      yaw: Number(viewport.getAttribute("data-globe-yaw")),
      zoom: Number(viewport.getAttribute("data-camera-zoom")),
    } : null;
  })()`);
  if (!before) throw new Error("the rotated World camera state is unavailable");
  const pointer = {
    x: before.bounds.x + before.bounds.width / 2,
    y: before.bounds.y + before.bounds.height / 2,
  };
  const animationFrames = await connection.evaluate(`new Promise((resolve) => {
    const viewport = document.querySelector('[role="application"]');
    const samples = [];
    const sample = () => {
      const projection = document.querySelector('[data-projection-morph-progress]');
      const regime = document.querySelector('[data-lens-regime]')?.getAttribute('data-lens-regime');
      const canvas = document.querySelector('canvas[data-globe-horizontal-wrap-opacity]');
      samples.push({
        sampled_at_ms: performance.now(),
        progress: projection
          ? Number(projection.getAttribute('data-projection-morph-progress'))
          : regime === 'atlas' ? 1 : 0,
        regime,
        repeat_depth: Number(canvas?.getAttribute('data-globe-horizontal-wrap-depth') ?? '0'),
        repeat_opacity: Number(canvas?.getAttribute('data-globe-horizontal-wrap-opacity') ?? '0'),
        atmosphere_opacity: Number(canvas?.getAttribute('data-globe-atmosphere-opacity') ?? '0'),
        atmosphere_repeat_opacity: Number(canvas?.getAttribute('data-globe-atmosphere-repeat-opacity') ?? '0'),
        atmosphere_shell_scale: Number(canvas?.getAttribute('data-globe-atmosphere-shell-scale') ?? '0'),
        surface_opacity: Number(canvas?.getAttribute('data-globe-surface-opacity') ?? '0'),
        zoom: Number(viewport?.getAttribute('data-camera-zoom')),
      });
      if (samples.length >= 120 || regime === 'atlas') resolve(samples);
      else requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
    for (let index = 0; index < 8; index += 1) {
      setTimeout(() => {
        viewport?.dispatchEvent(new WheelEvent('wheel', {
          bubbles: true,
          cancelable: true,
          clientX: ${pointer.x},
          clientY: ${pointer.y},
          deltaX: 0,
          deltaY: -50,
          view: window,
        }));
      }, index * 120);
    }
  })`);
  await waitFor(
    connection,
    `document.querySelector('[data-lens-regime="atlas"]')?.dataset.lensRegime === "atlas"`,
    "rotated World-to-Atlas wheel unfurl",
    timeoutMs,
  );
  await sleep(400);
  const after = await connection.evaluate(`(() => {
    const viewport = document.querySelector('[role="application"]');
    const canvas = document.querySelector('canvas[data-globe-horizontal-wrap-indexes]');
    const scene = canvas?.parentElement;
    const canvasBounds = canvas?.getBoundingClientRect();
    const sceneBounds = scene?.getBoundingClientRect();
    return {
      atlas_view_offset: document.querySelector('[data-atlas-view-offset]')?.getAttribute('data-atlas-view-offset') ?? null,
      horizontal_wrap_indexes: canvas?.getAttribute('data-globe-horizontal-wrap-indexes') ?? null,
      horizontal_wrap_depth: Number(canvas?.getAttribute('data-globe-horizontal-wrap-depth') ?? 'NaN'),
      horizontal_wrap_opacity: Number(canvas?.getAttribute('data-globe-horizontal-wrap-opacity') ?? 'NaN'),
      horizontal_wrap_layout: canvasBounds && sceneBounds ? {
        canvas_center_x: canvasBounds.x + canvasBounds.width / 2,
        canvas_width: canvasBounds.width,
        scene_center_x: sceneBounds.x + sceneBounds.width / 2,
        scene_width: sceneBounds.width,
      } : null,
      zoom: Number(viewport?.getAttribute('data-camera-zoom')),
    };
  })()`);
  const exitAnimationFrames = await sampleWheelProjectionTransition(
    connection,
    pointer,
    25,
    "world",
  );
  await waitFor(
    connection,
    `document.querySelector('[data-lens-regime="world"]')?.dataset.lensRegime === "world"`,
    "Atlas-to-World repeat dissolve",
    timeoutMs,
  );
  await sleep(400);
  const returnAnimationFrames = await sampleWheelProjectionTransition(
    connection,
    pointer,
    -25,
    "atlas",
  );
  await waitFor(
    connection,
    `document.querySelector('[data-lens-regime="atlas"]')?.dataset.lensRegime === "atlas"`,
    "World-to-Atlas return after reverse dissolve",
    timeoutMs,
  );
  await sleep(400);
  const distinctIntermediateFrames = new Set(
    animationFrames
      .filter(({ progress }) => progress > 0 && progress < 1)
      .map(({ progress }) => progress.toFixed(3)),
  ).size;
  const enteringDissolveFrames = new Set(
    returnAnimationFrames
      .filter(({ repeat_opacity: opacity }) => opacity > 0 && opacity < 1)
      .map(({ repeat_opacity: opacity }) => opacity.toFixed(3)),
  ).size;
  const exitingDissolveFrames = new Set(
    exitAnimationFrames
      .filter(({ repeat_opacity: opacity }) => opacity > 0 && opacity < 1)
      .map(({ repeat_opacity: opacity }) => opacity.toFixed(3)),
  ).size;
  const enteringPresentationGaps = animationFrames
    .slice(1)
    .map(
      ({ sampled_at_ms: sampledAt }, index) =>
        sampledAt - animationFrames[index].sampled_at_ms,
    );
  const enteringDepthFrames = returnAnimationFrames.filter(
    ({ repeat_depth: depth, repeat_opacity: opacity }) =>
      opacity > 0 && opacity < 1 && depth < 0,
  ).length;
  const exitingDepthFrames = exitAnimationFrames.filter(
    ({ repeat_depth: depth, repeat_opacity: opacity }) =>
      opacity > 0 && opacity < 1 && depth < 0,
  ).length;
  const enteringDissolveMonotonic = returnAnimationFrames.every(
    ({ repeat_opacity: opacity }, index) =>
      index === 0 || opacity >= returnAnimationFrames[index - 1].repeat_opacity,
  );
  const exitingDissolveMonotonic = exitAnimationFrames.every(
    ({ repeat_opacity: opacity }, index) =>
      index === 0 || opacity <= exitAnimationFrames[index - 1].repeat_opacity,
  );
  const atmospherePostureConsistent = [
    ...animationFrames,
    ...exitAnimationFrames,
    ...returnAnimationFrames,
  ].every((frame) => {
    const progress = Math.max(0, Math.min(1, frame.progress));
    const morphRemaining = 1 - progress * progress * (3 - 2 * progress);
    const expectedOpacity = morphRemaining * morphRemaining;
    const expectedRepeatOpacity =
      frame.repeat_opacity * (1 - frame.repeat_opacity);
    const expectedShellScale = Math.sqrt(morphRemaining);
    const surfaceFadeProgress = Math.max(
      0,
      Math.min(1, (progress - 0.38) / (0.62 - 0.38)),
    );
    const expectedSurfaceOpacity =
      morphRemaining *
      (1 -
        surfaceFadeProgress *
          surfaceFadeProgress *
          (3 - 2 * surfaceFadeProgress));
    return (
      Number.isFinite(frame.atmosphere_opacity) &&
      Number.isFinite(frame.atmosphere_repeat_opacity) &&
      Number.isFinite(frame.atmosphere_shell_scale) &&
      Number.isFinite(frame.surface_opacity) &&
      // DOM diagnostics are retained to three decimal places. The expected
      // curve is recomputed from a separately rounded progress attribute.
      Math.abs(frame.atmosphere_opacity - expectedOpacity) <= 0.003 &&
      Math.abs(frame.atmosphere_repeat_opacity - expectedRepeatOpacity) <=
        0.003 &&
      Math.abs(frame.atmosphere_shell_scale - expectedShellScale) <= 0.003 &&
      Math.abs(frame.surface_opacity - expectedSurfaceOpacity) <= 0.003
    );
  });
  return {
    after,
    animation_frames: animationFrames,
    atmosphere_posture_consistent: atmospherePostureConsistent,
    before,
    closing_atmosphere_screenshot: null,
    distinct_intermediate_frames: distinctIntermediateFrames,
    depth_transition_screenshot: null,
    entering_repeat_dissolve_frames: enteringDissolveFrames,
    entering_repeat_depth_frames: enteringDepthFrames,
    entering_presentation_max_gap_ms:
      enteringPresentationGaps.length > 0
        ? Math.max(...enteringPresentationGaps)
        : 0,
    exit_animation_frames: exitAnimationFrames,
    exiting_repeat_dissolve_frames: exitingDissolveFrames,
    exiting_repeat_depth_frames: exitingDepthFrames,
    return_animation_frames: returnAnimationFrames,
    observed:
      Math.abs(before.yaw) > 0.001 &&
      distinctIntermediateFrames >= 3 &&
      enteringDissolveFrames >= 2 &&
      enteringDepthFrames >= 1 &&
      enteringDissolveMonotonic &&
      atmospherePostureConsistent &&
      exitingDissolveFrames >= 2 &&
      exitingDepthFrames >= 1 &&
      exitingDissolveMonotonic &&
      animationFrames.at(-1)?.regime === "atlas" &&
      exitAnimationFrames.at(-1)?.regime === "world" &&
      after.atlas_view_offset !== null &&
      after.atlas_view_offset !== "0,0" &&
      after.horizontal_wrap_indexes === "-1,0,1" &&
      after.horizontal_wrap_depth === 0 &&
      after.horizontal_wrap_opacity === 1 &&
      after.horizontal_wrap_layout !== null &&
      Math.abs(
        after.horizontal_wrap_layout.canvas_center_x -
          after.horizontal_wrap_layout.scene_center_x,
      ) < 1 &&
      Math.abs(
        after.horizontal_wrap_layout.canvas_width -
          after.horizontal_wrap_layout.scene_width * 3,
      ) < 1,
  };
}

async function sampleWheelProjectionTransition(
  connection,
  pointer,
  deltaY,
  targetRegime,
) {
  return connection.evaluate(`new Promise((resolve) => {
    const viewport = document.querySelector('[role="application"]');
    const samples = [];
    let dispatched = 0;
    const read = () => {
      const projection = document.querySelector('[data-projection-morph-progress]');
      const regime = document.querySelector('[data-lens-regime]')?.getAttribute('data-lens-regime');
      const canvas = document.querySelector('canvas[data-globe-horizontal-wrap-opacity]');
      return {
        sampled_at_ms: performance.now(),
        progress: projection
          ? Number(projection.getAttribute('data-projection-morph-progress'))
          : regime === 'atlas' ? 1 : 0,
        regime,
        repeat_depth: Number(canvas?.getAttribute('data-globe-horizontal-wrap-depth') ?? '0'),
        repeat_opacity: Number(canvas?.getAttribute('data-globe-horizontal-wrap-opacity') ?? '0'),
        atmosphere_opacity: Number(canvas?.getAttribute('data-globe-atmosphere-opacity') ?? '0'),
        atmosphere_repeat_opacity: Number(canvas?.getAttribute('data-globe-atmosphere-repeat-opacity') ?? '0'),
        atmosphere_shell_scale: Number(canvas?.getAttribute('data-globe-atmosphere-shell-scale') ?? '0'),
        surface_opacity: Number(canvas?.getAttribute('data-globe-surface-opacity') ?? '0'),
        zoom: Number(viewport?.getAttribute('data-camera-zoom')),
      };
    };
    const wheelTimer = setInterval(() => {
      viewport?.dispatchEvent(new WheelEvent('wheel', {
        bubbles: true,
        cancelable: true,
        clientX: ${pointer.x},
        clientY: ${pointer.y},
        deltaX: 0,
        deltaY: ${deltaY},
        view: window,
      }));
      dispatched += 1;
      if (dispatched >= 24) clearInterval(wheelTimer);
    }, 120);
    const sample = () => {
      const frame = read();
      samples.push(frame);
      if (frame.regime === ${JSON.stringify(targetRegime)} || samples.length >= 360) {
        clearInterval(wheelTimer);
        resolve(samples);
      } else requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  })`);
}

function regimeExpression(regime) {
  return `document.querySelector('[data-lens-regime="${regime}"]')?.dataset.lensRegime === "${regime}"`;
}

async function waitForSubmittedTerrainFrame(connection, timeoutMs) {
  await waitFor(
    connection,
    `(() => {
      const snapshot = document.querySelector('[data-scene-snapshot]')?.getAttribute('data-scene-snapshot');
      const diagnostics = document.querySelector('[data-renderer-diagnostics]');
      return snapshot !== null && snapshot !== undefined &&
        diagnostics?.getAttribute('data-renderer-submitted-snapshot-id') === snapshot;
    })()`,
    "terrain renderer submission for the current semantic scene",
    timeoutMs,
  );
}

async function waitForAtlasTerrainPrewarm(connection, timeoutMs) {
  await waitFor(
    connection,
    `(() => {
      const state = document.querySelector('[data-atlas-terrain-prewarm]')
        ?.getAttribute('data-atlas-terrain-prewarm');
      return state === 'submitted' || state === 'unavailable';
    })()`,
    "idle Atlas terrain prewarm submission",
    timeoutMs,
  );
}

async function captureStage(connection, voyageDirectory, stage, startedAt) {
  const evidence = await connection.evaluate(`(() => {
    const shell = document.querySelector("[data-scene-snapshot]");
    const projection = document.querySelector("[data-lens-regime]");
    const diagnostics = document.querySelector("[data-renderer-diagnostics]");
    const geographicCoordinate = diagnostics?.querySelector("[data-coordinate-authority]");
    const footer = document.querySelector("[data-explorer-footer]");
    const globeCaption = document.querySelector("[data-globe-caption]");
    const globeAtmosphere = document.querySelector("[data-globe-atmosphere]");
    const globeSphere = document.querySelector("[data-globe-sphere]");
    const diagnosticsBounds = diagnostics?.getBoundingClientRect();
    const footerBounds = footer?.getBoundingClientRect();
    const globeCaptionBounds = globeCaption?.getBoundingClientRect();
    const renderedGlobeAtmosphereBounds = globeAtmosphere?.getBoundingClientRect();
    const globeSphereBounds = globeSphere?.getBoundingClientRect();
    const globeHaloScale = Number(globeSphere?.getAttribute("data-globe-halo-scale"));
    let sceneOmissions = [];
    try {
      sceneOmissions = JSON.parse(shell?.getAttribute("data-scene-omissions") ?? "[]");
    } catch {}
    const globeAtmosphereBounds = renderedGlobeAtmosphereBounds ?? (
      globeSphereBounds && Number.isFinite(globeHaloScale)
        ? {
            x: globeSphereBounds.x - globeSphereBounds.width * (globeHaloScale - 1) / 2,
            y: globeSphereBounds.y - globeSphereBounds.height * (globeHaloScale - 1) / 2,
            width: globeSphereBounds.width * globeHaloScale,
            height: globeSphereBounds.height * globeHaloScale,
          }
        : null
    );
    const globePoles = [...document.querySelectorAll("[data-globe-pole-pattern]")].map((pattern) => {
      const pole = pattern.getAttribute("data-globe-pole-pattern");
      return {
        label_count: pattern.querySelectorAll("text").length,
        pole,
        sample_count: Number(pattern.getAttribute("data-globe-pole-sample-count")),
      };
    });
    const exactEvidence = [...document.querySelectorAll("[data-object-evidence]")].map((element) => ({
      href: element.getAttribute("href"),
      identity: element.getAttribute("data-semantic-identity"),
      label: element.getAttribute("aria-label"),
      uri: element.getAttribute("data-object-evidence"),
    }));
    return {
      captured_at_unix_ms: Date.now(),
      atlas_terrain_prewarm: shell?.getAttribute("data-atlas-terrain-prewarm") ?? null,
      compilers: shell?.getAttribute("data-scene-compilers") ?? null,
      focus_id: shell?.getAttribute("data-scene-focus") ?? null,
      projection: projection ? {
        regime: projection.getAttribute("data-lens-regime"),
        render_graph_id: projection.getAttribute("data-render-graph"),
        render_passes: projection.getAttribute("data-render-passes")?.split(",").filter(Boolean) ?? [],
        renderer: projection.getAttribute("data-renderer"),
      } : null,
      renderer: diagnostics ? Object.fromEntries([...diagnostics.attributes]
        .filter((attribute) => attribute.name.startsWith("data-renderer-"))
        .map((attribute) => [attribute.name.slice("data-renderer-".length).replaceAll("-", "_"), attribute.value])) : null,
      renderer_diagnostics_text: diagnostics?.textContent?.replace(/\s+/g, " ").trim() ?? null,
      geographic_coordinate: geographicCoordinate ? {
        authority: geographicCoordinate.getAttribute("data-coordinate-authority"),
        text: geographicCoordinate.textContent?.replace(/\s+/g, " ").trim() ?? null,
      } : null,
      communication_layout: {
        diagnostics_bottom_gap_px: diagnosticsBounds ? innerHeight - diagnosticsBounds.bottom : null,
        footer_height_px: footerBounds?.height ?? null,
        footer_visible: footer?.getAttribute("data-visible") === "true",
      },
      globe_caption: globeCaption && globeCaptionBounds && globeAtmosphereBounds ? {
        horizontal_offset_from_globe_center_px:
          globeCaptionBounds.x + globeCaptionBounds.width / 2 -
          (globeAtmosphereBounds.x + globeAtmosphereBounds.width / 2),
        text: globeCaption.textContent?.replace(/\s+/g, " ").trim() ?? null,
        vertical_gap_from_atmosphere_px:
          globeCaptionBounds.y -
          (globeAtmosphereBounds.y + globeAtmosphereBounds.height),
      } : null,
      globe_poles: globeAtmosphereBounds ? {
        atmosphere_center_y:
          globeAtmosphereBounds.y + globeAtmosphereBounds.height / 2,
        patterns: globePoles,
      } : null,
      scene_compilation_ms: Number(shell?.getAttribute("data-scene-compilation-ms") ?? "NaN"),
      scene_snapshot_id: shell?.getAttribute("data-scene-snapshot") ?? null,
      source_revisions: shell?.getAttribute("data-scene-sources")?.split(",").filter(Boolean) ?? [],
      scene_omissions: sceneOmissions,
      labels: [...document.querySelectorAll("[data-label-disposition]")].reduce((counts, element) => {
        const disposition = element.getAttribute("data-label-disposition") ?? "unknown";
        counts[disposition] = (counts[disposition] ?? 0) + 1;
        counts.total += 1;
        return counts;
      }, { total: 0 }),
      mailbox_count: Number(
        document.querySelector('[aria-label="Open mailbox history"] span:last-child')?.textContent ?? "NaN",
      ),
      exact_evidence_links: exactEvidence,
      url: window.location.href,
      viewport: { width: innerWidth, height: innerHeight, device_pixel_ratio: devicePixelRatio },
    };
  })()`);
  const frameCadence = await connection.evaluate(`new Promise((resolve) => {
    const timestamps = [];
    const sample = (timestamp) => {
      timestamps.push(timestamp);
      if (timestamps.length < 25) requestAnimationFrame(sample);
      else {
        const intervals = timestamps.slice(1).map((value, index) => value - timestamps[index]);
        const sorted = [...intervals].sort((left, right) => left - right);
        const percentile = (fraction) => sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1)] ?? 0;
        resolve({
          authority: "browser requestAnimationFrame presentation cadence; not GPU execution duration",
          frames: intervals.length,
          maximum_ms: Math.max(...intervals),
          median_ms: percentile(0.5),
          p95_ms: percentile(0.95),
        });
      }
    };
    requestAnimationFrame(sample);
  })`);
  const cdpMetrics = await connection.send("Performance.getMetrics");
  const metrics = Object.fromEntries(
    cdpMetrics.metrics.map(({ name, value }) => [name, value]),
  );
  const performanceMetrics = {
    authority:
      "Chrome process metrics sampled after stage convergence; durations are cumulative seconds since navigation",
    js_heap_total_bytes: metrics.JSHeapTotalSize ?? null,
    js_heap_used_bytes: metrics.JSHeapUsedSize ?? null,
    layout_duration_seconds: metrics.LayoutDuration ?? null,
    script_duration_seconds: metrics.ScriptDuration ?? null,
    task_duration_seconds: metrics.TaskDuration ?? null,
  };
  const response = await connection.send("Page.captureScreenshot", {
    captureBeyondViewport: false,
    format: "png",
    fromSurface: true,
  });
  const image = Buffer.from(response.data, "base64");
  const file = `${String(CAPTURE_ORDER.indexOf(stage) + 1).padStart(2, "0")}-${stage}.png`;
  await writeFile(join(voyageDirectory, file), image);
  return {
    ...evidence,
    elapsed_ms: Number((performance.now() - startedAt).toFixed(3)),
    frame_cadence: frameCadence,
    performance_metrics: performanceMetrics,
    screenshot: { bytes: image.byteLength, file, sha256: sha256(image) },
    stage,
  };
}

function attentionRevalidationDocument(portfolio) {
  const document = structuredClone(portfolio);
  document.attention.attention_id = `${portfolio.attention.attention_id}:qualification-revalidation`;
  document.attention.source_snapshot_id = `${portfolio.attention.source_snapshot_id}:qualification-revalidation`;
  document.attention.rows.push({
    action: "create",
    dependency_ids: [],
    estimated_cost_units: 1,
    evidence_ids: ["qualification:passive-revalidation"],
    priority: 1,
    readiness: "ready",
    reason: "bounded passive-revalidation qualification stimulus",
    row_id: "qualification:passive-revalidation",
    subject_id: "qualification:passive-revalidation",
    subject_kind: "surface",
  });
  document.attention.summary.create += 1;
  document.attention.summary.surfaces += 1;
  document.attention.summary.unowned_surfaces += 1;
  return document;
}

async function runVoyage(options) {
  const browser = findBrowser(options.browser);
  const browserVersion = spawnSync(browser, ["--version"], {
    encoding: "utf8",
  }).stdout.trim();
  const portfolioResponse = await fetch(`${options.baseUrl}/api/v1/workloads`, {
    headers: { Accept: "application/json" },
  });
  if (!portfolioResponse.ok) {
    throw new Error(
      `workload evidence request failed (${portfolioResponse.status})`,
    );
  }
  const portfolio = await portfolioResponse.json();
  if (portfolio.schema !== "rey.workload-list.v1") {
    throw new Error(`unexpected workload evidence schema ${portfolio.schema}`);
  }
  const admittedRegion = portfolio.semantic_atlas?.regional_regions?.find(
    (region) => region.scene_region_id === options.region,
  );
  if (!admittedRegion) {
    throw new Error(
      `no admitted semantic-atlas region named ${options.region}`,
    );
  }

  let landscapeWorkloadBinding = null;
  if (options.landscapeWorkload) {
    const bytes = await readFile(LANDSCAPE_WORKLOAD_SUITE);
    const suite = validateLandscapeWorkloadSuite(JSON.parse(bytes));
    landscapeWorkloadBinding = {
      path: LANDSCAPE_WORKLOAD_SUITE,
      sha256: sha256(bytes),
      suite_id: suite.suite_id,
      workload: landscapeWorkload(
        suite,
        options.landscapeWorkload,
        `${options.width}x${options.height}`,
      ),
    };
  }

  const voyageName = `world-atlas-county-evidence-${options.backend}-${options.width}x${options.height}${landscapeWorkloadBinding ? `-${landscapeWorkloadBinding.workload.id}` : ""}${options.loss === "none" ? "" : `-${options.loss}`}${options.revalidation === "none" ? "" : `-${options.revalidation}-revalidation`}`;
  const voyageDirectory = join(
    options.outputRoot,
    `${timestamp()}-${voyageName}`,
  );
  await mkdir(voyageDirectory, { recursive: true });
  const route = new URL("/explore", options.baseUrl);
  route.searchParams.set("renderer", options.backend);
  let fulfilledDocuments = null;
  if (options.transport === "fulfilled") {
    const paths = [
      "/api/v1/health",
      "/api/v1/channels",
      "/api/v1/observations",
      "/api/v1/revalidation",
      "/api/v1/workloads/evidence",
      "/api/v1/conversations",
    ];
    fulfilledDocuments = Object.fromEntries(
      await Promise.all(
        paths.map(async (path) => {
          const response = await fetch(`${options.baseUrl}${path}`, {
            headers: { Accept: "application/json" },
          });
          if (!response.ok) {
            throw new Error(
              `qualification bootstrap request failed for ${path} (${response.status})`,
            );
          }
          return [path, await response.json()];
        }),
      ),
    );
    fulfilledDocuments["/api/v1/workloads"] =
      options.revalidation === "attention"
        ? {
            rey_qualification_sequence: true,
            responses: [portfolio, attentionRevalidationDocument(portfolio)],
          }
        : portfolio;
  }
  const launched = await launchChrome(
    browser,
    options.backend,
    route,
    fulfilledDocuments,
  );
  const { connection } = launched;
  const consoleEntries = [];
  const exceptions = [];
  connection.on("Log.entryAdded", ({ entry }) => consoleEntries.push(entry));
  connection.on("Runtime.consoleAPICalled", ({ type, args, timestamp: at }) => {
    if (type === "error" || type === "warning") {
      consoleEntries.push({
        level: type,
        source: "console-api",
        text: args
          .map((argument) => argument.value ?? argument.description ?? "")
          .join(" "),
        timestamp: at,
      });
    }
  });
  connection.on("Runtime.exceptionThrown", ({ exceptionDetails }) => {
    exceptions.push({
      text: exceptionDetails.exception?.description ?? exceptionDetails.text,
      timestamp: exceptionDetails.timestamp,
      url: exceptionDetails.url,
    });
  });

  let failure = null;
  let failureContext = null;
  const captures = [];
  const interactions = [];
  let passiveRevalidationBaseline = null;
  let passiveRevalidationObserved = null;
  let onboardingNoticeObserved = false;
  let firstInteractionDismissalObserved = false;
  let mapNoticeObserved = false;
  let outsideGlobePan = null;
  let rotatedWorldAtlasUnfurl = null;
  let smoothWorldWheel = null;
  const startedAt = performance.now();
  const startedAtUnixMs = Date.now();
  try {
    process.stdout.write(`START ${voyageName}\n`);
    await Promise.all([
      connection.send("Page.enable"),
      connection.send("Performance.enable"),
      connection.send("Runtime.enable"),
    ]);
    await connection.send("Emulation.setDeviceMetricsOverride", {
      deviceScaleFactor: options.devicePixelRatio,
      height: options.height,
      mobile: false,
      width: options.width,
    });

    if (options.transport === "direct") {
      await connection.send("Log.enable");
      await connection.send("Page.navigate", { url: route.href });
    } else {
      await connection.send("Page.navigate", { url: launched.bootstrapUrl });
      await connection.send("Log.enable");
    }
    await waitFor(
      connection,
      `document.querySelector('[data-lens-regime]')`,
      "initial Explorer projection",
      options.timeoutMs,
    );
    onboardingNoticeObserved = await connection.evaluate(`(() => {
      const footer = document.querySelector('[data-explorer-footer]');
      const header = document.querySelector('[data-explorer-canvas-header]');
      const diagnostics = document.querySelector('[data-renderer-diagnostics]');
      const style = footer ? getComputedStyle(footer) : null;
      const headerStyle = header ? getComputedStyle(header) : null;
      const footerBounds = footer?.getBoundingClientRect();
      const diagnosticsBounds = diagnostics?.getBoundingClientRect();
      return footer?.dataset.visible === "true" &&
        footer?.dataset.noticeTone === "guide" &&
        footer?.textContent?.includes("WHEEL / + − TO CHANGE LENS · DRAG TO ORBIT · SELECT TO TRAVERSE") === true &&
        style?.justifyContent === "center" &&
        style?.backgroundColor !== "rgba(0, 0, 0, 0)" &&
        style?.backgroundColor === headerStyle?.backgroundColor &&
        diagnosticsBounds && footerBounds && diagnosticsBounds.bottom <= footerBounds.top + 1 &&
        style?.transitionProperty.includes("transform") === true;
    })()`);
    if (!onboardingNoticeObserved)
      throw new Error("the centered Explorer onboarding notice is unavailable");
    if (!(await connection.evaluate(regimeExpression("world")))) {
      await dispatchClick(
        connection,
        `document.querySelector('[aria-label="Zoom out one semantic level"]')`,
        "World zoom control",
        options.timeoutMs,
      );
    }
    await waitFor(
      connection,
      regimeExpression("world"),
      "World projection",
      options.timeoutMs,
    );
    process.stdout.write("READY world projection\n");
    await waitFor(
      connection,
      `document.querySelector('[data-renderer-diagnostics]')?.dataset.rendererBackend === "${options.backend}" && document.querySelector('[data-renderer-diagnostics]')?.dataset.rendererLifecycle !== "initializing"`,
      `${options.backend} renderer selection`,
      options.timeoutMs,
    );
    process.stdout.write(`READY world / ${options.backend}\n`);
    await measureInteraction(
      interactions,
      "smooth_world_wheel_zoom",
      async () => {
        smoothWorldWheel = await verifySmoothWorldWheelZoom(
          connection,
          options.timeoutMs,
        );
      },
    );
    if (!smoothWorldWheel?.observed)
      throw new Error(
        "wheel zoom did not animate through distinct pointer-anchored frames",
      );
    await measureInteraction(
      interactions,
      "pan_outside_world_globe",
      async () => {
        outsideGlobePan = await panOutsideGlobe(connection);
      },
    );
    if (!outsideGlobePan?.observed)
      throw new Error(
        "dragging outside the World atmosphere did not pan without orbiting",
      );
    if (options.revalidation === "attention") {
      passiveRevalidationBaseline = await connection.evaluate(`Number(
        document.querySelector('[aria-label="Open mailbox history"] span:last-child')?.textContent ?? "NaN"
      )`);
      if (!Number.isFinite(passiveRevalidationBaseline))
        throw new Error(
          "the passive-revalidation mailbox baseline is unavailable",
        );
    }

    const region = JSON.stringify(options.region);
    const worldRegion = `[...document.querySelectorAll('[data-semantic-region]')].find((element) => element.getAttribute('aria-label')?.startsWith(${region} + ':'))`;
    process.stdout.write(`ROTATE world / ${options.region}\n`);
    await measureInteraction(
      interactions,
      "rotate_world_to_region",
      async () => {
        await rotateGlobeToRegion(
          connection,
          admittedRegion.semantic_longitude_microdegrees,
        );
        await sleep(50);
        await waitFor(
          connection,
          worldRegion,
          `${options.region} visible World marker`,
          options.timeoutMs,
        );
      },
    );
    captures.push(
      await captureStage(connection, voyageDirectory, "world", startedAt),
    );
    if (options.loss !== "none") {
      const induced = await connection.evaluate(
        options.loss === "webgl-context"
          ? `(() => {
              const canvas = document.querySelector('canvas[data-renderer="react-three-fiber"]');
              const context = canvas?.getContext("webgl2");
              const extension = context?.getExtension("WEBGL_lose_context");
              if (!extension) return false;
              extension.loseContext();
              return true;
            })()`
          : `document.querySelector('canvas[data-renderer="react-three-fiber"]')
              ?.dispatchEvent(new CustomEvent("rey:qualify-webgpu-device-loss")) === true`,
      );
      if (!induced) throw new Error(`${options.loss} could not be induced`);
      await waitFor(
        connection,
        `document.querySelector('[data-renderer-diagnostics]')?.dataset.rendererBackend === "reference" && document.querySelector('[data-renderer-diagnostics]')?.dataset.rendererDegraded === "true" && document.querySelector('[data-renderer-diagnostics]')?.dataset.rendererLifecycle === "failed"`,
        `${options.loss} visible reference fallback`,
        options.timeoutMs,
      );
      process.stdout.write(`READY backend loss / ${options.loss}\n`);
      captures.push(
        await captureStage(
          connection,
          voyageDirectory,
          "backend-loss",
          startedAt,
        ),
      );
    }
    await measureInteraction(interactions, "world_to_atlas", async () => {
      rotatedWorldAtlasUnfurl = await verifyRotatedWorldAtlasUnfurl(
        connection,
        options.timeoutMs,
      );
    });
    if (!rotatedWorldAtlasUnfurl?.observed)
      throw new Error(
        "rotated World-to-Atlas wheel input did not retain a view-aligned unfurl",
      );
    process.stdout.write("READY atlas\n");
    firstInteractionDismissalObserved = await connection.evaluate(`(() => {
      const footer = document.querySelector('[data-explorer-footer]');
      return footer?.textContent?.includes("WHEEL / + − TO CHANGE LENS") === false;
    })()`);
    if (!firstInteractionDismissalObserved)
      throw new Error(
        "the Explorer onboarding notice survived map interaction",
      );
    if (options.backend !== "reference")
      await waitForAtlasTerrainPrewarm(connection, options.timeoutMs);
    captures.push(
      await captureStage(connection, voyageDirectory, "atlas", startedAt),
    );

    await measureInteraction(interactions, "atlas_to_county", async () => {
      await dispatchClick(
        connection,
        `document.querySelector('[role="button"][data-chart-wrap-index="0"][data-semantic-identity]')`,
        "canonical Atlas region",
        options.timeoutMs,
      );
      await waitFor(
        connection,
        regimeExpression("landscape"),
        "County landscape",
        options.timeoutMs,
      );
    });
    mapNoticeObserved = await connection.evaluate(`(() => {
      const footer = document.querySelector('[data-explorer-footer]');
      return footer?.dataset.visible === "true" &&
        footer?.dataset.noticeTone === "update" &&
        footer?.textContent?.includes("FOCUS /") === true;
    })()`);
    if (!mapNoticeObserved)
      throw new Error("the Explorer focus notice did not resurface");
    if (options.backend !== "reference")
      await waitForSubmittedTerrainFrame(connection, options.timeoutMs);
    process.stdout.write("READY landscape\n");
    captures.push(
      await captureStage(connection, voyageDirectory, "landscape", startedAt),
    );

    const firstProjectionButton = `document.querySelector('[data-lens-regime] [role="button"][aria-label]')`;
    await measureInteraction(
      interactions,
      "county_to_neighborhoods",
      async () => {
        await dispatchClick(
          connection,
          firstProjectionButton,
          "County landscape object",
          options.timeoutMs,
        );
        await waitFor(
          connection,
          regimeExpression("neighborhoods"),
          "County neighborhoods",
          options.timeoutMs,
        );
      },
    );
    await measureInteraction(
      interactions,
      "neighborhoods_to_objects",
      async () => {
        await dispatchClick(
          connection,
          firstProjectionButton,
          "County neighborhood object",
          options.timeoutMs,
        );
        await waitFor(
          connection,
          regimeExpression("objects"),
          "County objects",
          options.timeoutMs,
        );
      },
    );
    if (options.backend !== "reference")
      await waitForSubmittedTerrainFrame(connection, options.timeoutMs);
    process.stdout.write("READY objects\n");
    captures.push(
      await captureStage(connection, voyageDirectory, "objects", startedAt),
    );

    await measureInteraction(interactions, "objects_to_evidence", async () => {
      await dispatchClick(
        connection,
        `document.querySelector('[aria-label="Zoom in one semantic level"]')`,
        "Evidence zoom control",
        options.timeoutMs,
      );
      await waitFor(
        connection,
        regimeExpression("evidence"),
        "exact Evidence projection",
        options.timeoutMs,
      );
      await waitFor(
        connection,
        `document.querySelectorAll('[data-object-evidence]').length > 0`,
        "exact regional evidence links",
        options.timeoutMs,
      );
    });
    if (options.backend !== "reference")
      await waitForSubmittedTerrainFrame(connection, options.timeoutMs);
    process.stdout.write("READY evidence\n");
    captures.push(
      await captureStage(connection, voyageDirectory, "evidence", startedAt),
    );
    if (options.revalidation === "attention") {
      await waitFor(
        connection,
        `(globalThis.__reyQualificationFetchCounts?.["/api/v1/workloads"] ?? 0) >= 2 && Number(document.querySelector('[aria-label="Open mailbox history"] span:last-child')?.textContent ?? "NaN") === ${passiveRevalidationBaseline + 1}`,
        "passive workload attention revalidation",
        options.timeoutMs,
      );
      passiveRevalidationObserved = true;
      process.stdout.write("READY passive revalidation / attention\n");
      captures.push(
        await captureStage(
          connection,
          voyageDirectory,
          "passive-revalidation",
          startedAt,
        ),
      );
    }
  } catch (error) {
    failure = error instanceof Error ? error.message : String(error);
    try {
      const [documentContext, frameTree] = await Promise.all([
        connection.evaluate(`({
          body_text: document.body?.textContent?.replace(/\\s+/g, " ").trim().slice(0, 2_000) ?? null,
          document_ready_state: document.readyState,
          renderer: (() => {
            const diagnostics = document.querySelector('[data-renderer-diagnostics]');
            return diagnostics ? Object.fromEntries([...diagnostics.attributes]
              .filter((attribute) => attribute.name.startsWith('data-renderer-'))
              .map((attribute) => [attribute.name, attribute.value])) : null;
          })(),
          title: document.title,
          url: window.location.href,
        })`),
        connection.send("Page.getFrameTree"),
      ]);
      failureContext = { ...documentContext, frame_tree: frameTree };
    } catch (contextError) {
      failureContext = {
        inspection_error:
          contextError instanceof Error
            ? contextError.message
            : String(contextError),
      };
    }
  } finally {
    connection.close();
    if (launched.child.exitCode === null) {
      launched.child.kill("SIGTERM");
      await Promise.race([
        new Promise((resolvePromise) =>
          launched.child.once("exit", resolvePromise),
        ),
        sleep(5_000),
      ]);
    }
    try {
      await rm(launched.profile, {
        force: true,
        maxRetries: 5,
        recursive: true,
        retryDelay: 100,
      });
    } catch (error) {
      consoleEntries.push({
        level: "warning",
        source: "qualification-cleanup",
        text: `temporary browser profile cleanup failed: ${error instanceof Error ? error.message : String(error)}`,
      });
    }
  }

  const world = captures.find((capture) => capture.stage === "world");
  const expectedStagesPresent = STAGES.every((stage) =>
    captures.some((capture) => capture.stage === stage),
  );
  const backendMatched = world?.renderer?.backend === options.backend;
  const worldCaptionCentered =
    world?.globe_caption !== null &&
    world?.globe_caption !== undefined &&
    Math.abs(world.globe_caption.horizontal_offset_from_globe_center_px) < 1 &&
    world.globe_caption.vertical_gap_from_atmosphere_px >= 10 &&
    world.globe_caption.text?.includes("ADMITTED REGIONS") === false;
  const northPole = world?.globe_poles?.patterns.find(
    ({ pole }) => pole === "north",
  );
  const southPole = world?.globe_poles?.patterns.find(
    ({ pole }) => pole === "south",
  );
  const patternedGlobePolesPresent =
    northPole?.sample_count === 34 &&
    southPole?.sample_count === 34 &&
    northPole.label_count === 0 &&
    southPole.label_count === 0;
  const exactEvidencePresent =
    (captures.find((capture) => capture.stage === "evidence")
      ?.exact_evidence_links.length ?? 0) > 0;
  const lossFallbackObserved =
    options.loss === "none"
      ? null
      : captures.some(
          (capture) =>
            capture.stage === "backend-loss" &&
            capture.renderer?.backend === "reference" &&
            capture.renderer?.degraded === "true" &&
            capture.renderer?.lifecycle === "failed",
        );
  const landscapeCapture = captures.find(
    (capture) => capture.stage === "landscape",
  );
  const landscapeWorkloadEvaluation = landscapeWorkloadBinding
    ? evaluateLandscapeCapture(
        landscapeCapture,
        landscapeWorkloadBinding.workload,
        options,
        lossFallbackObserved,
      )
    : null;
  const sceneIdentityRetained =
    captures.length > 0 &&
    new Set(captures.map((capture) => capture.scene_snapshot_id)).size === 5;
  const geographicCoordinatesPresent = captures.every(
    (capture) =>
      capture.geographic_coordinate?.authority &&
      /^LAT -?\d+\.\d{4}° \/ LON -?\d+\.\d{4}°$/.test(
        capture.geographic_coordinate.text ?? "",
      ),
  );
  const compactNavigationDiagnosticsPresent = captures.every((capture) =>
    /^ZOOM \d+%LAT -?\d+\.\d{4}° \/ LON -?\d+\.\d{4}°$/.test(
      capture.renderer_diagnostics_text ?? "",
    ),
  );
  const nativeCountyCoordinatesPresent = captures
    .filter((capture) =>
      ["landscape", "objects", "evidence"].includes(capture.stage),
    )
    .every(
      (capture) => capture.geographic_coordinate?.authority === "native_crs84",
    );
  const hiddenWorldLayout = captures.find(
    (capture) => capture.stage === "world",
  )?.communication_layout;
  const visibleMapNoticeLayout = captures.find(
    (capture) =>
      ["landscape", "objects", "evidence"].includes(capture.stage) &&
      capture.communication_layout.footer_visible,
  )?.communication_layout;
  const diagnosticsFollowFooter =
    hiddenWorldLayout?.footer_visible === false &&
    visibleMapNoticeLayout?.footer_visible === true &&
    Number.isFinite(hiddenWorldLayout.diagnostics_bottom_gap_px) &&
    Number.isFinite(visibleMapNoticeLayout.diagnostics_bottom_gap_px) &&
    visibleMapNoticeLayout.diagnostics_bottom_gap_px >
      hiddenWorldLayout.diagnostics_bottom_gap_px + 24;
  const expectedLossConsoleEntries = consoleEntries.filter((entry) =>
    expectedLossConsoleEntry(entry, options.loss),
  );
  const unexpectedConsoleErrors = consoleEntries.filter(
    (entry) =>
      entry.level === "error" && !expectedLossConsoleEntry(entry, options.loss),
  );
  const complete =
    !failure &&
    expectedStagesPresent &&
    backendMatched &&
    worldCaptionCentered &&
    patternedGlobePolesPresent &&
    exactEvidencePresent &&
    geographicCoordinatesPresent &&
    compactNavigationDiagnosticsPresent &&
    nativeCountyCoordinatesPresent &&
    diagnosticsFollowFooter &&
    onboardingNoticeObserved &&
    firstInteractionDismissalObserved &&
    mapNoticeObserved &&
    smoothWorldWheel?.observed === true &&
    outsideGlobePan?.observed === true &&
    rotatedWorldAtlasUnfurl?.observed === true &&
    landscapeWorkloadEvaluation?.passed !== false &&
    lossFallbackObserved !== false &&
    passiveRevalidationObserved !== false &&
    exceptions.length === 0 &&
    unexpectedConsoleErrors.length === 0;
  const omissions = [
    ...(complete
      ? []
      : [failure ?? "voyage qualification assertions did not converge"]),
    ...(admittedRegion.native_objects > 0 &&
    admittedRegion.terrain_objects === 0
      ? [
          "the admitted regional fixture has no terrain object; accelerated World globe capture does not claim County terrain acceleration",
        ]
      : []),
    ...(!landscapeWorkloadBinding
      ? [
          "named Landscape fidelity requirements were not selected; add --landscape-workload to qualify a Plan 0005 fixture",
        ]
      : landscapeWorkloadEvaluation?.passed
        ? []
        : [
            `Landscape workload ${landscapeWorkloadBinding.workload.id} did not satisfy every retained requirement`,
          ]),
    "captures do not measure GPU execution duration or frame rate",
    ...(options.loss === "none"
      ? [
          "device-loss and WebGL context-loss injection require separately named voyages",
        ]
      : []),
    ...(options.revalidation === "none"
      ? ["passive revalidation requires a separately named voyage"]
      : [
          "passive revalidation used one bounded qualification-generated attention row, not observed runtime activity",
        ]),
  ];
  const manifest = {
    schema: SCHEMA,
    authority: AUTHORITY,
    voyage: {
      name: voyageName,
      complete,
      started_at_unix_ms: startedAtUnixMs,
      elapsed_ms: Number((performance.now() - startedAt).toFixed(3)),
      warm_posture:
        "cold isolated browser profile; each later stage reuses the same page and renderer lifecycle",
      transport_posture:
        options.transport === "fulfilled"
          ? "Chrome HTTP requests were fulfilled from bounded reads of the named rey agent origin because direct browser sockets were unavailable in the qualification environment"
          : "Chrome fetched the named rey agent origin directly",
    },
    request: {
      backend: options.backend,
      base_url: options.baseUrl,
      device_pixel_ratio: options.devicePixelRatio,
      height: options.height,
      landscape_workload: options.landscapeWorkload,
      loss: options.loss,
      region: options.region,
      revalidation: options.revalidation,
      timeout_ms: options.timeoutMs,
      transport: options.transport,
      width: options.width,
    },
    machine: {
      architecture: process.arch,
      browser: basename(browser),
      browser_version: browserVersion,
      cpu_model: cpus()[0]?.model ?? "unknown",
      hostname: hostname(),
      operating_system: platform(),
      release: release(),
    },
    inputs: {
      admitted_region: admittedRegion,
      atlas_revision: portfolio.semantic_atlas.atlas_revision,
      attention_snapshot_id: portfolio.attention.source_snapshot_id,
      workload_revision:
        portfolio.revision?.head?.snapshot?.snapshot_revision ?? null,
      landscape_workload: landscapeWorkloadBinding
        ? {
            path: landscapeWorkloadBinding.path,
            sha256: landscapeWorkloadBinding.sha256,
            suite_id: landscapeWorkloadBinding.suite_id,
            workload_id: landscapeWorkloadBinding.workload.id,
          }
        : null,
    },
    assertions: {
      backend_matched: backendMatched,
      centered_world_caption_observed: worldCaptionCentered,
      patterned_globe_poles_present: patternedGlobePolesPresent,
      exact_evidence_present: exactEvidencePresent,
      expected_stages_present: expectedStagesPresent,
      diagnostics_follow_footer: diagnosticsFollowFooter,
      compact_navigation_diagnostics_present:
        compactNavigationDiagnosticsPresent,
      smooth_world_wheel_zoom_observed: smoothWorldWheel?.observed ?? false,
      first_interaction_dismissal_observed: firstInteractionDismissalObserved,
      expected_loss_console_entries: expectedLossConsoleEntries.length,
      loss_fallback_observed: lossFallbackObserved,
      map_notice_observed: mapNoticeObserved,
      geographic_coordinates_present: geographicCoordinatesPresent,
      native_county_coordinates_present: nativeCountyCoordinatesPresent,
      no_browser_exceptions: exceptions.length === 0,
      no_unexpected_console_errors: unexpectedConsoleErrors.length === 0,
      onboarding_notice_observed: onboardingNoticeObserved,
      outside_globe_pan_observed: outsideGlobePan?.observed ?? false,
      rotated_world_atlas_unfurl_observed:
        rotatedWorldAtlasUnfurl?.observed ?? false,
      landscape_workload_passed: landscapeWorkloadEvaluation?.passed ?? null,
      passive_revalidation_observed: passiveRevalidationObserved,
      scene_snapshot_changed_with_each_semantic_stage: sceneIdentityRetained,
    },
    captures,
    interactions,
    world_wheel_zoom: smoothWorldWheel,
    rotated_world_atlas_unfurl: rotatedWorldAtlasUnfurl,
    world_drag_partition: outsideGlobePan,
    landscape_workload: landscapeWorkloadEvaluation,
    revalidation:
      options.revalidation === "attention"
        ? {
            authority:
              "bounded generated attention stimulus for passive UI refresh qualification only; not observed runtime activity or admitted evidence",
            baseline_mailbox_count: passiveRevalidationBaseline,
            mode: options.revalidation,
            observed_mailbox_count: captures.find(
              ({ stage }) => stage === "passive-revalidation",
            )?.mailbox_count,
          }
        : null,
    browser_exceptions: exceptions,
    console_entries: consoleEntries,
    failure_context: failureContext,
    initial_target_url: launched.initialTargetUrl,
    omissions,
    chrome_stderr_tail: launched.stderr().slice(-4_000),
  };
  const serialized = `${JSON.stringify(manifest, null, 2)}\n`;
  await writeFile(join(voyageDirectory, "manifest.json"), serialized);
  const retained = await readFile(join(voyageDirectory, "manifest.json"));
  process.stdout.write(
    `${complete ? "PASS" : "INCOMPLETE"} ${voyageName}\n${voyageDirectory}/manifest.json\n${sha256(retained)}\n`,
  );
  if (!complete) process.exitCode = 1;
}

let options;
try {
  options = parseArguments(process.argv.slice(2));
  if (options.help) {
    process.stdout.write(`${usage()}\n`);
  } else {
    await runVoyage(options);
  }
} catch (error) {
  process.stderr.write(
    `${error instanceof Error ? error.message : String(error)}\n\n${usage()}\n`,
  );
  process.exitCode = 2;
}
