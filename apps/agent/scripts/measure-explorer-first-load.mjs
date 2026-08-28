#!/usr/bin/env node

import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";

const PENDING_MARK = "rey.explorer.pending-canvas";
const READY_MARK = "rey.explorer.scene-ready";
const DIST_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../dist");

function argumentsFrom(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--") continue;
    if (!argument.startsWith("--"))
      throw new Error(`unexpected argument ${argument}`);
    const name = argument.slice(2);
    const value = argv[index + 1];
    if (!value || value.startsWith("--"))
      throw new Error(`--${name} requires a value`);
    values.set(name, value);
    index += 1;
  }
  const positiveNumber = (name, fallback) => {
    const value = Number(values.get(name) ?? fallback);
    if (!Number.isFinite(value) || value <= 0)
      throw new Error(`--${name} must be a positive number`);
    return value;
  };
  const baseUrl = new URL(
    values.get("base-url") ?? "http://127.0.0.1:5714/explore",
  );
  if (!/^https?:$/.test(baseUrl.protocol))
    throw new Error("--base-url must use HTTP or HTTPS");
  const transport = values.get("transport") ?? "direct";
  if (!new Set(["direct", "fulfilled"]).has(transport))
    throw new Error("--transport must be direct or fulfilled");
  return {
    baseUrl: baseUrl.href,
    browser: values.get("browser") ?? null,
    pendingBudgetMs: positiveNumber("pending-budget-ms", 2_000),
    timeoutMs: positiveNumber("timeout-ms", 30_000),
    transport,
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
    const found = spawnSync("which", [candidate], { encoding: "utf8" });
    if (found.status === 0 && found.stdout.trim()) return found.stdout.trim();
  }
  throw new Error("Chrome or Chromium was not found; pass --browser PATH");
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

class CdpConnection {
  #id = 0;
  #pending = new Map();
  #socket;

  static async connect(url) {
    const socket = new WebSocket(url);
    await new Promise((resolve, reject) => {
      socket.addEventListener("open", resolve, { once: true });
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
      const message = JSON.parse(String(event.data));
      if (message.id === undefined) return;
      const pending = this.#pending.get(message.id);
      if (!pending) return;
      this.#pending.delete(message.id);
      if (message.error) pending.reject(new Error(message.error.message));
      else pending.resolve(message.result);
    });
    socket.addEventListener("close", () => {
      for (const pending of this.#pending.values())
        pending.reject(new Error("Chrome DevTools websocket closed"));
      this.#pending.clear();
    });
  }

  send(method, params = {}) {
    const id = (this.#id += 1);
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { reject, resolve });
      this.#socket.send(JSON.stringify({ id, method, params }));
    });
  }

  async evaluate(expression) {
    const result = await this.send("Runtime.evaluate", {
      awaitPromise: true,
      expression,
      returnByValue: true,
    });
    if (result.exceptionDetails)
      throw new Error(
        result.exceptionDetails.exception?.description ??
          result.exceptionDetails.text ??
          "browser evaluation failed",
      );
    return result.result.value;
  }

  close() {
    this.#socket.close();
  }
}

async function waitFor(connection, expression, description, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await connection.evaluate(`Boolean(${expression})`)) return;
    await delay(50);
  }
  throw new Error(`timed out waiting for ${description}`);
}

async function launchBrowser(browser, profile, allowFileAccess) {
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
    "--use-gl=angle",
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
  ];
  if (allowFileAccess) flags.push("--allow-file-access-from-files");
  flags.push("about:blank");
  const child = spawn(browser, flags, { stdio: ["ignore", "ignore", "pipe"] });
  let stderr = "";
  const devtoolsUrl = await new Promise((resolve, reject) => {
    const timer = setTimeout(
      () =>
        reject(
          new Error(`Chrome did not expose DevTools: ${stderr.slice(-1_000)}`),
        ),
      15_000,
    );
    child.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
      const match = /DevTools listening on (ws:\/\/[^\s]+)/.exec(stderr);
      if (!match) return;
      clearTimeout(timer);
      resolve(match[1]);
    });
    child.once("exit", (code) => {
      clearTimeout(timer);
      reject(new Error(`Chrome exited before DevTools was ready (${code})`));
    });
  });
  const debugOrigin = devtoolsUrl
    .replace(/^ws:/, "http:")
    .replace(/\/devtools\/browser\/.*$/, "");
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const response = await fetch(`${debugOrigin}/json/list`);
    const targets = response.ok ? await response.json() : [];
    const target = targets.find(
      (candidate) =>
        candidate.type === "page" && candidate.webSocketDebuggerUrl,
    );
    if (target)
      return {
        child,
        connection: await CdpConnection.connect(target.webSocketDebuggerUrl),
        stderr: () => stderr,
      };
    await delay(100);
  }
  throw new Error("Chrome did not expose the Explorer page target");
}

async function fulfilledBootstrap(baseUrl, profile) {
  const origin = new URL(baseUrl).origin;
  const paths = [
    "/api/v1/health",
    "/api/v1/observations",
    "/api/v1/conversations",
    "/api/v1/channels",
    "/api/v1/revalidation",
    "/api/v1/workloads",
  ];
  const startedAt = performance.now();
  const documents = Object.fromEntries(
    await Promise.all(
      paths.map(async (path) => {
        const requestStartedAt = performance.now();
        const response = await fetch(`${origin}${path}`, {
          headers: { Accept: "application/json" },
        });
        const body = await response.text();
        if (!response.ok)
          throw new Error(`${path} failed (${response.status}): ${body}`);
        return [
          path,
          {
            body: JSON.parse(body),
            elapsed_ms: performance.now() - requestStartedAt,
          },
        ];
      }),
    ),
  );
  const workloadDelayMs = documents["/api/v1/workloads"].elapsed_ms;
  const serializedDocuments = JSON.stringify(
    Object.fromEntries(
      Object.entries(documents).map(([path, document]) => [
        path,
        document.body,
      ]),
    ),
  ).replaceAll("<", "\\u003c");
  const distributionRoot = `${pathToFileURL(DIST_ROOT).href}/`;
  let html = await readFile(join(DIST_ROOT, "index.html"), "utf8");
  html = html.replaceAll('="/assets/', `="${distributionRoot}assets/`);
  const bootstrap = `<script>
    const documents = ${serializedDocuments};
    const workloadDelayMs = ${JSON.stringify(workloadDelayMs)};
    globalThis.EventSource = undefined;
    globalThis.fetch = async (input) => {
      const value = typeof input === "string" ? input : input.url;
      const pathname = value.startsWith("/")
        ? new URL(value, "http://rey.measurement").pathname
        : new URL(value).pathname;
      if (pathname === "/api/v1/workloads")
        await new Promise((resolve) => setTimeout(resolve, workloadDelayMs));
      const document = documents[pathname];
      if (document === undefined)
        return new Response("measurement bootstrap has no retained response", { status: 404 });
      return new Response(JSON.stringify(document), {
        headers: { "Content-Type": "application/json; charset=utf-8" },
        status: 200,
      });
    };
  </script>`;
  html = html.replace("</head>", `${bootstrap}</head>`);
  const bootstrapPath = join(profile, "explore.html");
  await writeFile(bootstrapPath, html);
  return {
    pageUrl: pathToFileURL(bootstrapPath).href,
    server_projection_ms: workloadDelayMs,
    source_read_ms: performance.now() - startedAt,
  };
}

async function main() {
  const options = argumentsFrom(process.argv.slice(2));
  const browser = findBrowser(options.browser);
  const profile = await mkdtemp(join(tmpdir(), "rey-explorer-first-load-"));
  let launched;
  try {
    const fulfilled =
      options.transport === "fulfilled"
        ? await fulfilledBootstrap(options.baseUrl, profile)
        : null;
    const pageUrl = fulfilled?.pageUrl ?? options.baseUrl;
    launched = await launchBrowser(
      browser,
      profile,
      options.transport === "fulfilled",
    );
    const { connection } = launched;
    await Promise.all([
      connection.send("Page.enable"),
      connection.send("Runtime.enable"),
    ]);
    await connection.send("Page.navigate", { url: pageUrl });
    try {
      await waitFor(
        connection,
        `performance.getEntriesByName(${JSON.stringify(PENDING_MARK)}).length > 0`,
        "the evidence-pending Explorer canvas",
        options.timeoutMs,
      );
    } catch (error) {
      const diagnostic = await connection.evaluate(`({
        body: document.body?.innerText?.slice(0, 1_000) ?? null,
        ready_state: document.readyState,
        title: document.title,
        url: location.href,
      })`);
      throw new Error(`${error.message}: ${JSON.stringify(diagnostic)}`);
    }
    const pending = await connection.evaluate(`(() => {
      const mark = performance.getEntriesByName(${JSON.stringify(PENDING_MARK)}).at(-1);
      const canvas = document.querySelector('[data-scene-projection]');
      return {
        mark_ms: mark?.startTime ?? null,
        projection: canvas?.getAttribute('data-scene-projection') ?? null,
        county_visible: document.body.textContent?.includes('Rey County') ?? false,
      };
    })()`);
    await waitFor(
      connection,
      `performance.getEntriesByName(${JSON.stringify(READY_MARK)}).length > 0`,
      "the admitted Explorer scene",
      options.timeoutMs,
    );
    const ready = await connection.evaluate(`(() => {
      const mark = performance.getEntriesByName(${JSON.stringify(READY_MARK)}).at(-1);
      const workload = performance.getEntriesByType('resource').find((entry) => new URL(entry.name).pathname === '/api/v1/workloads');
      const canvas = document.querySelector('[data-scene-snapshot]');
      return {
        mark_ms: mark?.startTime ?? null,
        scene_compilation_ms: Number(canvas?.getAttribute('data-scene-compilation-ms')),
        scene_phases_ms: {
          topology: Number(canvas?.getAttribute('data-scene-topology-ms')),
          freeze: Number(canvas?.getAttribute('data-scene-freeze-ms')),
          render_graph: Number(canvas?.getAttribute('data-scene-render-graph-ms')),
          picking: Number(canvas?.getAttribute('data-scene-picking-ms')),
          identity: Number(canvas?.getAttribute('data-scene-identity-ms')),
        },
        topology_phases_ms: {
          admission: Number(canvas?.getAttribute('data-scene-topology-admission-ms')),
          regional_landscape: Number(canvas?.getAttribute('data-scene-topology-regional-landscape-ms')),
          atlas_landscape: Number(canvas?.getAttribute('data-scene-topology-atlas-landscape-ms')),
          projection: Number(canvas?.getAttribute('data-scene-topology-projection-ms')),
          atlas_prewarm: Number(canvas?.getAttribute('data-scene-topology-atlas-prewarm-ms')),
          world_atlas: Number(canvas?.getAttribute('data-scene-topology-world-atlas-ms')),
          regional_frame: Number(canvas?.getAttribute('data-scene-topology-regional-frame-ms')),
          regional_fields: Number(canvas?.getAttribute('data-scene-topology-regional-fields-ms')),
          regional_mosaic: Number(canvas?.getAttribute('data-scene-topology-regional-mosaic-ms')),
        },
        snapshot_id: canvas?.getAttribute('data-scene-snapshot') ?? null,
        workload_response_end_ms: workload?.responseEnd ?? null,
        workload_duration_ms: workload?.duration ?? null,
        workload_transfer_bytes: workload?.transferSize ?? null,
      };
    })()`);
    const passed =
      pending.projection === "pending" &&
      pending.county_visible === false &&
      Number.isFinite(pending.mark_ms) &&
      pending.mark_ms <= options.pendingBudgetMs &&
      Number.isFinite(ready.mark_ms) &&
      ready.mark_ms >= pending.mark_ms &&
      typeof ready.snapshot_id === "string" &&
      ready.snapshot_id.length > 0;
    const result = {
      schema: "rey.explorer-first-load-measurement.v1",
      workload: "cold HTTP /explore → pending canvas → admitted scene",
      url: options.baseUrl,
      browser,
      transport: {
        posture: options.transport,
        server_projection_ms: fulfilled?.server_projection_ms ?? null,
        source_read_ms: fulfilled?.source_read_ms ?? null,
      },
      pending_budget_ms: options.pendingBudgetMs,
      pending,
      ready,
      passed,
    };
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    if (!passed) process.exitCode = 2;
  } finally {
    if (launched) {
      await launched.connection.send("Browser.close").catch(() => undefined);
      launched.connection.close();
      launched.child.kill("SIGTERM");
    }
    await rm(profile, { force: true, recursive: true });
  }
}

await main();
