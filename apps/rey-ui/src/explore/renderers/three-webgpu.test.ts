import { describe, expect, it, vi } from "vitest";
import {
  ReactThreeFiberRendererAdapter,
  type ThreeRendererFacade,
} from "./three-webgpu";

function rendererFacade(backend: "webgpu" | "webgl2") {
  return {
    backend: {
      isWebGPUBackend: backend === "webgpu",
      isWebGLBackend: backend === "webgl2",
    },
    info: { render: { calls: 7 } },
    init: vi.fn(async () => undefined),
    setPixelRatio: vi.fn(),
    setSize: vi.fn(),
    render: vi.fn(),
    dispose: vi.fn(),
  } satisfies ThreeRendererFacade;
}

describe("React Three Fiber WebGPU renderer adapter", () => {
  it("prefers WebGPU and instruments Fiber-owned frame submission", async () => {
    const renderer = rendererFacade("webgpu");
    const render = renderer.render;
    const factory = vi.fn(async () => renderer);
    const adapter = new ReactThreeFiberRendererAdapter(factory);
    const submitted = vi.fn();
    adapter.onFrameSubmitted(submitted);

    const status = await adapter.initialize({} as HTMLCanvasElement, "auto");
    expect(factory).toHaveBeenCalledWith({
      canvas: expect.anything(),
      forceWebGL: false,
    });
    expect(status).toMatchObject({
      lifecycle: "ready",
      backend: "webgpu",
      degraded: false,
    });
    expect(renderer.init).toHaveBeenCalledOnce();
    adapter.renderer?.render({}, {});
    expect(render).toHaveBeenCalledOnce();
    expect(submitted).toHaveBeenCalledOnce();
    expect(adapter.lastDrawCalls).toBe(7);
    expect(adapter.lastSubmissionMs).toBeGreaterThanOrEqual(0);
  });

  it("forces Three.js's WebGL2 compatibility backend for qualification", async () => {
    const renderer = rendererFacade("webgl2");
    const factory = vi.fn(async () => renderer);
    const adapter = new ReactThreeFiberRendererAdapter(factory);

    const status = await adapter.initialize({} as HTMLCanvasElement, "webgl2");
    expect(factory).toHaveBeenCalledWith({
      canvas: expect.anything(),
      forceWebGL: true,
    });
    expect(status).toMatchObject({
      lifecycle: "ready",
      backend: "webgl2",
      degraded: false,
    });
  });

  it("fails closed to reference status when required WebGPU is unavailable", async () => {
    const renderer = rendererFacade("webgl2");
    const adapter = new ReactThreeFiberRendererAdapter(async () => renderer);

    const status = await adapter.initialize({} as HTMLCanvasElement, "webgpu");
    expect(status).toMatchObject({
      lifecycle: "failed",
      backend: "reference",
      degraded: true,
    });
    expect(renderer.dispose).toHaveBeenCalledOnce();
  });

  it("disposes a partially initialized renderer when initialization fails", async () => {
    const renderer = rendererFacade("webgpu");
    renderer.init.mockRejectedValueOnce(new Error("adapter unavailable"));
    const adapter = new ReactThreeFiberRendererAdapter(async () => renderer);

    const status = await adapter.initialize({} as HTMLCanvasElement);
    expect(status).toMatchObject({
      lifecycle: "failed",
      backend: "reference",
      degraded: true,
      detail: "adapter unavailable",
    });
    expect(renderer.dispose).toHaveBeenCalledOnce();
  });

  it("disposes resources and rejects reuse", async () => {
    const renderer = rendererFacade("webgpu");
    const adapter = new ReactThreeFiberRendererAdapter(async () => renderer);
    await adapter.initialize({} as HTMLCanvasElement);
    adapter.dispose();

    expect(renderer.dispose).toHaveBeenCalledOnce();
    expect(adapter.status.lifecycle).toBe("disposed");
    expect(adapter.lastDrawCalls).toBe(0);
    await expect(adapter.initialize({} as HTMLCanvasElement)).rejects.toThrow(
      "disposed",
    );
  });

  it("reports WebGPU device loss as a visible reference fallback", async () => {
    let loseDevice!: (info: { reason?: string; message?: string }) => void;
    const renderer = rendererFacade("webgpu");
    Object.assign(renderer.backend, {
      device: {
        lost: new Promise<{ reason?: string; message?: string }>((resolve) => {
          loseDevice = resolve;
        }),
      },
    });
    const adapter = new ReactThreeFiberRendererAdapter(async () => renderer);
    const statuses: string[] = [];
    adapter.onStatusChange(({ detail }) => statuses.push(detail));
    await adapter.initialize({} as HTMLCanvasElement);

    loseDevice({ reason: "destroyed", message: "qualification fixture" });
    await Promise.resolve();

    expect(adapter.status).toMatchObject({
      lifecycle: "failed",
      backend: "reference",
      degraded: true,
    });
    expect(statuses.at(-1)).toContain("qualification fixture");
    expect(renderer.dispose).toHaveBeenCalledOnce();
  });

  it("destroys only a ready WebGPU device through the qualification hook", async () => {
    let loseDevice!: (info: { reason?: string; message?: string }) => void;
    const renderer = rendererFacade("webgpu");
    const destroy = vi.fn(() =>
      loseDevice({ reason: "destroyed", message: "qualification hook" }),
    );
    Object.assign(renderer.backend, {
      device: {
        destroy,
        lost: new Promise<{ reason?: string; message?: string }>((resolve) => {
          loseDevice = resolve;
        }),
      },
    });
    const adapter = new ReactThreeFiberRendererAdapter(async () => renderer);

    expect(adapter.destroyWebGpuDeviceForQualification()).toBe(false);
    await adapter.initialize({} as HTMLCanvasElement, "webgpu");
    expect(adapter.destroyWebGpuDeviceForQualification()).toBe(true);
    await Promise.resolve();

    expect(destroy).toHaveBeenCalledOnce();
    expect(adapter.status).toMatchObject({
      lifecycle: "failed",
      backend: "reference",
      degraded: true,
    });
    expect(adapter.destroyWebGpuDeviceForQualification()).toBe(false);
  });
});
