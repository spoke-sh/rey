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
  maximumDimension = 2048,
  maximumPhysicalPixels = 8_388_608,
): RendererViewport {
  const devicePixelRatio = Math.min(
    maximumDevicePixelRatio,
    Math.max(1, viewport.device_pixel_ratio),
  );
  const width = Math.max(1, Math.floor(viewport.width));
  const height = Math.max(1, Math.floor(viewport.height));
  const scale = Math.min(
    1,
    maximumDimension / width,
    maximumDimension / height,
    Math.sqrt(
      maximumPhysicalPixels /
        (width * height * devicePixelRatio * devicePixelRatio),
    ),
  );
  return {
    width: Math.max(1, Math.floor(width * scale)),
    height: Math.max(1, Math.floor(height * scale)),
    device_pixel_ratio: devicePixelRatio,
  };
}
