import type { Camera, Object3D } from "three/webgpu";
import {
  boundedViewport,
  renderFrameInvalidation,
  type AcceleratedBackend,
  type RenderFrameIdentity,
  type RendererStatus,
  type RendererViewport,
} from "../engine/renderer";

export const THREE_RENDERER_REVISION = "three@0.185.1:webgpu+tsl";

export interface ThreeRendererFacade {
  readonly backend: {
    readonly isWebGPUBackend?: boolean;
    readonly isWebGLBackend?: boolean;
    readonly device?: {
      readonly lost: Promise<{ reason?: string; message?: string }>;
    };
  };
  init(): Promise<unknown>;
  setPixelRatio(value: number): void;
  setSize(width: number, height: number, updateStyle?: boolean): void;
  render(scene: Object3D, camera: Camera): void;
  dispose(): void;
}

export type ThreeRendererFactory = (options: {
  canvas: HTMLCanvasElement;
  forceWebGL: boolean;
}) => Promise<ThreeRendererFacade>;

const defaultFactory: ThreeRendererFactory = async ({ canvas, forceWebGL }) => {
  const THREE = await import("three/webgpu");
  return new THREE.WebGPURenderer({
    alpha: true,
    antialias: true,
    canvas,
    forceWebGL,
  }) as unknown as ThreeRendererFacade;
};

export class ThreeWebGpuRendererAdapter {
  readonly #factory: ThreeRendererFactory;
  #renderer: ThreeRendererFacade | undefined;
  #status: RendererStatus = {
    lifecycle: "idle",
    backend: null,
    renderer_revision: THREE_RENDERER_REVISION,
    degraded: false,
    detail: "accelerated renderer has not been initialized",
  };
  #viewport: RendererViewport = {
    width: 1,
    height: 1,
    device_pixel_ratio: 1,
  };
  #lastFrame: RenderFrameIdentity | undefined;
  #lastSubmissionMs = 0;
  readonly #statusListeners = new Set<
    (status: Readonly<RendererStatus>) => void
  >();

  constructor(factory: ThreeRendererFactory = defaultFactory) {
    this.#factory = factory;
  }

  get status(): Readonly<RendererStatus> {
    return Object.freeze({ ...this.#status });
  }

  get lastFrame(): Readonly<RenderFrameIdentity> | undefined {
    return this.#lastFrame ? Object.freeze({ ...this.#lastFrame }) : undefined;
  }

  get lastSubmissionMs(): number {
    return this.#lastSubmissionMs;
  }

  onStatusChange(listener: (status: Readonly<RendererStatus>) => void) {
    this.#statusListeners.add(listener);
    return () => this.#statusListeners.delete(listener);
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
      detail: "initializing Three.js WebGPURenderer",
    };
    try {
      const forceWebGL = preferredBackend === "webgl2";
      const renderer = await this.#factory({ canvas, forceWebGL });
      this.#renderer = renderer;
      await renderer.init();
      this.applyViewport();
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
            ? "Three.js WebGPURenderer is using WebGPU"
            : forceWebGL
              ? "Three.js WebGL2 compatibility backend was forced for qualification"
              : "Three.js selected its WebGL2 compatibility backend",
      };
      if (backend === "webgpu" && renderer.backend.device)
        void renderer.backend.device.lost.then((info) => {
          if (this.#renderer !== renderer) return;
          renderer.dispose();
          this.#renderer = undefined;
          this.#lastFrame = undefined;
          this.#status = {
            lifecycle: "failed",
            backend: "reference",
            renderer_revision: THREE_RENDERER_REVISION,
            degraded: true,
            detail: `WebGPU device lost${info.reason ? ` (${info.reason})` : ""}${info.message ? `: ${info.message}` : ""}; the reference renderer remains active`,
          };
          this.notifyStatus();
        });
      return this.status;
    } catch (error) {
      this.#renderer?.dispose();
      this.#renderer = undefined;
      this.#status = {
        lifecycle: "failed",
        backend: "reference",
        renderer_revision: THREE_RENDERER_REVISION,
        degraded: true,
        detail: error instanceof Error ? error.message : String(error),
      };
      return this.status;
    }
  }

  resize(viewport: RendererViewport): void {
    this.#viewport = boundedViewport(viewport);
    if (this.#renderer) this.applyViewport();
  }

  render(scene: Object3D, camera: Camera, frame: RenderFrameIdentity): boolean {
    if (!this.#renderer || this.#status.lifecycle !== "ready")
      throw new Error("the Three.js renderer adapter is not ready");
    if (renderFrameInvalidation(this.#lastFrame, frame).length === 0)
      return false;
    const started = measurementNow();
    this.#renderer.render(scene, camera);
    this.#lastSubmissionMs = measurementNow() - started;
    this.#lastFrame = Object.freeze({ ...frame });
    return true;
  }

  dispose(): void {
    this.#renderer?.dispose();
    this.#renderer = undefined;
    this.#lastFrame = undefined;
    this.#lastSubmissionMs = 0;
    this.#status = {
      lifecycle: "disposed",
      backend: null,
      renderer_revision: THREE_RENDERER_REVISION,
      degraded: false,
      detail: "Three.js renderer resources have been disposed",
    };
  }

  private applyViewport(): void {
    if (!this.#renderer) return;
    this.#renderer.setPixelRatio(this.#viewport.device_pixel_ratio);
    this.#renderer.setSize(this.#viewport.width, this.#viewport.height, false);
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
