#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { cpus, hostname, platform, release, tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

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
const DIST_ROOT = join(REPOSITORY_ROOT, "apps/rey-ui/dist");

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

function regimeExpression(regime) {
  return `document.querySelector('[data-lens-regime="${regime}"]')?.dataset.lensRegime === "${regime}"`;
}

async function captureStage(connection, voyageDirectory, stage, startedAt) {
  const evidence = await connection.evaluate(`(() => {
    const shell = document.querySelector("[data-scene-snapshot]");
    const projection = document.querySelector("[data-lens-regime]");
    const diagnostics = document.querySelector("[data-renderer-diagnostics]");
    const exactEvidence = [...document.querySelectorAll("[data-object-evidence]")].map((element) => ({
      href: element.getAttribute("href"),
      identity: element.getAttribute("data-semantic-identity"),
      label: element.getAttribute("aria-label"),
      uri: element.getAttribute("data-object-evidence"),
    }));
    return {
      captured_at_unix_ms: Date.now(),
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
      scene_compilation_ms: Number(shell?.getAttribute("data-scene-compilation-ms") ?? "NaN"),
      scene_snapshot_id: shell?.getAttribute("data-scene-snapshot") ?? null,
      source_revisions: shell?.getAttribute("data-scene-sources")?.split(",").filter(Boolean) ?? [],
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

  const voyageName = `world-atlas-county-evidence-${options.backend}-${options.width}x${options.height}${options.loss === "none" ? "" : `-${options.loss}`}${options.revalidation === "none" ? "" : `-${options.revalidation}-revalidation`}`;
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
    if (!(await connection.evaluate(`Boolean(${worldRegion})`))) {
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
    }
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
      await dispatchClick(
        connection,
        worldRegion,
        `${options.region} World marker`,
        options.timeoutMs,
      );
      await waitFor(
        connection,
        regimeExpression("atlas"),
        "Atlas projection",
        options.timeoutMs,
      );
    });
    process.stdout.write("READY atlas\n");
    captures.push(
      await captureStage(connection, voyageDirectory, "atlas", startedAt),
    );

    await measureInteraction(interactions, "atlas_to_county", async () => {
      await dispatchClick(
        connection,
        `document.querySelector('button[data-chart-wrap-index="0"][data-semantic-identity]')`,
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
    process.stdout.write("READY landscape\n");
    captures.push(
      await captureStage(connection, voyageDirectory, "landscape", startedAt),
    );

    const firstProjectionButton = `document.querySelector('[data-lens-regime] button[aria-label]:not([disabled])')`;
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
  const sceneIdentityRetained =
    captures.length > 0 &&
    new Set(captures.map((capture) => capture.scene_snapshot_id)).size === 5;
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
    exactEvidencePresent &&
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
    },
    assertions: {
      backend_matched: backendMatched,
      exact_evidence_present: exactEvidencePresent,
      expected_stages_present: expectedStagesPresent,
      expected_loss_console_entries: expectedLossConsoleEntries.length,
      loss_fallback_observed: lossFallbackObserved,
      no_browser_exceptions: exceptions.length === 0,
      no_unexpected_console_errors: unexpectedConsoleErrors.length === 0,
      passive_revalidation_observed: passiveRevalidationObserved,
      scene_snapshot_changed_with_each_semantic_stage: sceneIdentityRetained,
    },
    captures,
    interactions,
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
