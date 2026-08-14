import { type AcceleratedBackend, type RendererStatus } from "./renderer";

export const THREE_RENDERER_REVISION =
  "@rey/explorer@0.1.0+react-three-fiber@9.7.0+three@0.185.1:webgpu+tsl+unified-runtime@1";
export const WEBGPU_DEVICE_LOSS_QUALIFICATION_EVENT =
  "rey:qualify-webgpu-device-loss";

export interface ThreeRendererFacade {
  readonly backend: {
    readonly isWebGPUBackend?: boolean;
    readonly isWebGLBackend?: boolean;
    readonly device?: {
      readonly lost: Promise<{ reason?: string; message?: string }>;
      destroy?(): void;
    };
  };
  readonly info?: {
    readonly render?: {
      readonly calls?: number;
    };
  };
  init(): Promise<unknown>;
  setPixelRatio(value: number): void;
  setSize(width: number, height: number, updateStyle?: boolean): void;
  render(scene: unknown, camera: unknown): void;
  dispose(): void;
}

export type ThreeRendererFactory = (options: {
  canvas: HTMLCanvasElement;
  forceWebGL: boolean;
}) => Promise<ThreeRendererFacade>;

const defaultFactory: ThreeRendererFactory = async ({ canvas, forceWebGL }) => {
  const { default: WebGPURenderer } =
    await import("three/src/renderers/webgpu/WebGPURenderer.js");
  return new WebGPURenderer({
    alpha: true,
    antialias: true,
    canvas,
    forceWebGL,
  }) as unknown as ThreeRendererFacade;
};

export class ReactThreeFiberRendererAdapter {
  readonly #factory: ThreeRendererFactory;
  #renderer: ThreeRendererFacade | undefined;
  #status: RendererStatus = {
    lifecycle: "idle",
    backend: null,
    renderer_revision: THREE_RENDERER_REVISION,
    degraded: false,
    detail: "accelerated renderer has not been initialized",
  };
  #lastDrawCalls = 0;
  #lastSubmissionMs = 0;
  readonly #statusListeners = new Set<
    (status: Readonly<RendererStatus>) => void
  >();
  readonly #frameListeners = new Set<() => void>();

  constructor(factory: ThreeRendererFactory = defaultFactory) {
    this.#factory = factory;
  }

  get status(): Readonly<RendererStatus> {
    return Object.freeze({ ...this.#status });
  }

  get renderer(): ThreeRendererFacade | undefined {
    return this.#renderer;
  }

  get lastSubmissionMs(): number {
    return this.#lastSubmissionMs;
  }

  get lastDrawCalls(): number {
    return this.#lastDrawCalls;
  }

  destroyWebGpuDeviceForQualification(): boolean {
    const device = this.#renderer?.backend.device;
    if (
      this.#status.lifecycle !== "ready" ||
      this.#status.backend !== "webgpu" ||
      !device?.destroy
    )
      return false;
    device.destroy();
    return true;
  }

  onStatusChange(listener: (status: Readonly<RendererStatus>) => void) {
    this.#statusListeners.add(listener);
    return () => this.#statusListeners.delete(listener);
  }

  onFrameSubmitted(listener: () => void) {
    this.#frameListeners.add(listener);
    return () => this.#frameListeners.delete(listener);
  }

  async initialize(
    canvas: HTMLCanvasElement,
    preferredBackend: "auto" | AcceleratedBackend = "auto",
  ): Promise<Readonly<RendererStatus>> {
    if (this.#status.lifecycle === "disposed")
      throw new Error("the Three.js renderer adapter has been disposed");
    if (this.#status.lifecycle === "ready") return this.status;
    this.#status = {
      ...this.#status,
      lifecycle: "initializing",
      detail: "initializing React Three Fiber WebGPURenderer",
    };
    this.notifyStatus();
    try {
      const forceWebGL = preferredBackend === "webgl2";
      const renderer = await this.#factory({ canvas, forceWebGL });
      this.#renderer = renderer;
      await renderer.init();
      this.instrumentRenderer(renderer);
      const backend = rendererBackend(renderer);
      if (preferredBackend === "webgpu" && backend !== "webgpu")
        throw new Error("WebGPU was required but Three.js selected WebGL2");
      this.#status = {
        lifecycle: "ready",
        backend,
        renderer_revision: THREE_RENDERER_REVISION,
        degraded: backend === "webgl2" && preferredBackend !== "webgl2",
        detail:
          backend === "webgpu"
            ? "React Three Fiber is rendering through Three.js WebGPU"
            : forceWebGL
              ? "React Three Fiber forced Three.js's WebGL2 compatibility backend for qualification"
              : "React Three Fiber is using Three.js's WebGL2 compatibility backend",
      };
      if (backend === "webgpu" && renderer.backend.device)
        void renderer.backend.device.lost.then((info) => {
          if (this.#renderer !== renderer) return;
          renderer.dispose();
          this.#renderer = undefined;
          this.#lastDrawCalls = 0;
          this.#status = {
            lifecycle: "failed",
            backend: "reference",
            renderer_revision: THREE_RENDERER_REVISION,
            degraded: true,
            detail: `WebGPU device lost${info.reason ? ` (${info.reason})` : ""}${info.message ? `: ${info.message}` : ""}; the reference renderer remains active`,
          };
          this.notifyStatus();
        });
      this.notifyStatus();
      return this.status;
    } catch (error) {
      this.#renderer?.dispose();
      this.#renderer = undefined;
      this.#lastDrawCalls = 0;
      this.#status = {
        lifecycle: "failed",
        backend: "reference",
        renderer_revision: THREE_RENDERER_REVISION,
        degraded: true,
        detail: error instanceof Error ? error.message : String(error),
      };
      this.notifyStatus();
      return this.status;
    }
  }

  dispose(): void {
    this.#renderer?.dispose();
    this.#renderer = undefined;
    this.#lastDrawCalls = 0;
    this.#lastSubmissionMs = 0;
    this.#status = {
      lifecycle: "disposed",
      backend: null,
      renderer_revision: THREE_RENDERER_REVISION,
      degraded: false,
      detail: "Three.js renderer resources have been disposed",
    };
  }

  private instrumentRenderer(renderer: ThreeRendererFacade): void {
    const render = renderer.render.bind(renderer);
    renderer.render = (scene, camera) => {
      const started = measurementNow();
      render(scene, camera);
      this.#lastSubmissionMs = measurementNow() - started;
      const drawCalls = renderer.info?.render?.calls;
      this.#lastDrawCalls =
        typeof drawCalls === "number" &&
        Number.isFinite(drawCalls) &&
        drawCalls >= 0
          ? Math.trunc(drawCalls)
          : 0;
      for (const listener of this.#frameListeners) listener();
    };
  }

  private notifyStatus(): void {
    const status = this.status;
    for (const listener of this.#statusListeners) listener(status);
  }
}

function measurementNow(): number {
  return globalThis.performance?.now() ?? Date.now();
}

function rendererBackend(renderer: ThreeRendererFacade): AcceleratedBackend {
  if (renderer.backend.isWebGPUBackend === true) return "webgpu";
  if (renderer.backend.isWebGLBackend === true) return "webgl2";
  throw new Error("Three.js initialized an unrecognized rendering backend");
}
