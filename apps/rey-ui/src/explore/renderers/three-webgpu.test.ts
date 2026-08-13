import type { Camera, Object3D } from "three/webgpu";
import { describe, expect, it, vi } from "vitest";
import {
  ThreeWebGpuRendererAdapter,
  type ThreeRendererFacade,
} from "./three-webgpu";

function rendererFacade(backend: "webgpu" | "webgl2") {
  return {
    backend: {
      isWebGPUBackend: backend === "webgpu",
      isWebGLBackend: backend === "webgl2",
    },
    init: vi.fn(async () => undefined),
    setPixelRatio: vi.fn(),
    setSize: vi.fn(),
    render: vi.fn(),
    dispose: vi.fn(),
  } satisfies ThreeRendererFacade;
}

describe("Three.js WebGPU renderer adapter", () => {
  it("prefers WebGPU and renders only after asynchronous initialization", async () => {
    const renderer = rendererFacade("webgpu");
    const factory = vi.fn(async () => renderer);
    const adapter = new ThreeWebGpuRendererAdapter(factory);
    adapter.resize({ width: 800.8, height: 600.4, device_pixel_ratio: 3 });

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
    expect(renderer.setPixelRatio).toHaveBeenCalledWith(2);
    expect(renderer.setSize).toHaveBeenCalledWith(800, 600, false);

    adapter.render({} as Object3D, {} as Camera, {
      snapshot_id: "scene:one",
      camera_revision: "camera:one",
      material_revision: "material:one",
      render_graph_id: "graph:one",
    });
    expect(renderer.render).toHaveBeenCalledOnce();
    expect(adapter.lastFrame?.snapshot_id).toBe("scene:one");
    expect(adapter.lastSubmissionMs).toBeGreaterThanOrEqual(0);
    expect(
      adapter.render({} as Object3D, {} as Camera, {
        snapshot_id: "scene:one",
        camera_revision: "camera:one",
        material_revision: "material:one",
        render_graph_id: "graph:one",
      }),
    ).toBe(false);
    expect(renderer.render).toHaveBeenCalledOnce();
  });

  it("forces Three.js's WebGL2 compatibility backend for qualification", async () => {
    const renderer = rendererFacade("webgl2");
    const factory = vi.fn(async () => renderer);
    const adapter = new ThreeWebGpuRendererAdapter(factory);

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
    const adapter = new ThreeWebGpuRendererAdapter(async () => renderer);

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
    const adapter = new ThreeWebGpuRendererAdapter(async () => renderer);

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
    const adapter = new ThreeWebGpuRendererAdapter(async () => renderer);
    await adapter.initialize({} as HTMLCanvasElement);
    adapter.dispose();

    expect(renderer.dispose).toHaveBeenCalledOnce();
    expect(adapter.status.lifecycle).toBe("disposed");
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
    const adapter = new ThreeWebGpuRendererAdapter(async () => renderer);
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
});
