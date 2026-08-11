export type AcceleratedBackend = "webgpu" | "webgl2";

export type RendererLifecycle =
  "idle" | "initializing" | "ready" | "failed" | "disposed";

export interface RendererViewport {
  width: number;
  height: number;
  device_pixel_ratio: number;
}

export interface RendererStatus {
  lifecycle: RendererLifecycle;
  backend: AcceleratedBackend | "reference" | null;
  renderer_revision: string;
  degraded: boolean;
  detail: string;
}

export interface RenderFrameIdentity {
  snapshot_id: string;
  camera_revision: string;
  material_revision: string;
}

export function boundedViewport(
  viewport: RendererViewport,
  maximumDevicePixelRatio = 2,
): RendererViewport {
  return {
    width: Math.max(1, Math.floor(viewport.width)),
    height: Math.max(1, Math.floor(viewport.height)),
    device_pixel_ratio: Math.min(
      maximumDevicePixelRatio,
      Math.max(1, viewport.device_pixel_ratio),
    ),
  };
}
