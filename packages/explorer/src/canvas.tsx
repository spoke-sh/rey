import { createRoot, type Renderer as FiberRenderer } from "@react-three/fiber";
import { useEffect, useRef, useState } from "react";
import { ContextGlobeScene, ContinuousReliefScene } from "./fiber-scenes";
import {
  boundedViewport,
  renderFrameInvalidation,
  type RenderFrameIdentity,
  type RendererStatus,
} from "./renderer";
import type { CompiledContextGlobe } from "./three-globe";
import type { CompiledContinuousRelief } from "./three-terrain";
import {
  GLOBE_ATLAS_HORIZONTAL_WRAP_INDEXES,
  globeAtlasRepeatDepthOffset,
  globeAtlasRepeatOpacity,
  globeAtmosphereOpacity,
  globeAtmosphereRepeatOpacity,
  globeAtmosphereShellScale,
  globeSurfaceOpacity,
} from "./globe-projection";
import {
  ReactThreeFiberRendererAdapter,
  THREE_RENDERER_REVISION,
  WEBGPU_DEVICE_LOSS_QUALIFICATION_EVENT,
} from "./three-webgpu";
import type { GlobeCameraView, TerrainCameraView } from "./types";

export type RendererPreference = "auto" | "webgpu" | "webgl2" | "reference";

export type ExplorerCanvasContent =
  | {
      kind: "globe";
      compiled: CompiledContextGlobe;
      view: GlobeCameraView;
      world: { width: number; height: number };
    }
  | {
      kind: "terrain";
      compiled: CompiledContinuousRelief;
      view: TerrainCameraView;
      world: { width: number; height: number };
    };

export interface ExplorerCanvasReport {
  status: Readonly<RendererStatus>;
  submitted_frame: Readonly<RenderFrameIdentity> | null;
  draw_calls: number;
  render_submission_ms: number;
}

export interface ExplorerCanvasProps {
  className?: string;
  content: ExplorerCanvasContent;
  frame: RenderFrameIdentity;
  onReport: (report: ExplorerCanvasReport) => void;
  opacity?: number;
  preference: RendererPreference;
  readyClassName?: string;
  visible: boolean;
}

const REFERENCE_STATUS: RendererStatus = Object.freeze({
  lifecycle: "ready",
  backend: "reference",
  renderer_revision: "rey.reference-renderer@1",
  degraded: false,
  detail: "the deterministic reference terrain is active",
});

export function ExplorerCanvas({
  className,
  content,
  frame,
  onReport,
  opacity = 1,
  preference,
  readyClassName,
  visible,
}: ExplorerCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rootRef = useRef<ReturnType<typeof createRoot> | undefined>(undefined);
  const adapterRef = useRef<ReactThreeFiberRendererAdapter | undefined>(
    undefined,
  );
  const lastFrameRef = useRef<RenderFrameIdentity | undefined>(undefined);
  const [rootGeneration, setRootGeneration] = useState(0);
  const [ready, setReady] = useState(false);
  const [submittedFrame, setSubmittedFrame] = useState<RenderFrameIdentity>();
  const horizontalWrapIndexes = globeHorizontalWrapIndexes(content);
  const horizontalWrapDepth = globeHorizontalWrapDepth(content);
  const horizontalWrapOpacity = globeHorizontalWrapOpacity(content);
  const viewport = canvasViewport(content);
  const wrappedCanvasStyle =
    content.kind === "globe" && horizontalWrapIndexes.length > 1
      ? {
          bottom: "auto",
          height: viewport.height,
          left: "50%",
          right: "auto",
          top: "50%",
          transform: `translate(-50%, -50%) scale(${(content.world.width * horizontalWrapIndexes.length) / viewport.width}, ${content.world.height / viewport.height})`,
          transformOrigin: "center center",
          width: viewport.width,
          opacity,
        }
      : { opacity };
  const onReportRef = useRef(onReport);
  onReportRef.current = onReport;
  const report = (
    status: Readonly<RendererStatus>,
    submittedFrame: Readonly<RenderFrameIdentity> | null = null,
  ) => {
    const adapter = adapterRef.current;
    onReportRef.current({
      status,
      submitted_frame: submittedFrame,
      draw_calls: adapter?.lastDrawCalls ?? 0,
      render_submission_ms: adapter?.lastSubmissionMs ?? 0,
    });
  };
  const reportRef = useRef(report);
  reportRef.current = report;
  const sceneElement =
    content.kind === "globe" ? (
      <ContextGlobeScene
        compiled={content.compiled}
        view={content.view}
        world={content.world}
      />
    ) : (
      <ContinuousReliefScene
        compiled={content.compiled}
        view={content.view}
        world={content.world}
      />
    );
  const sceneElementRef = useRef(sceneElement);
  sceneElementRef.current = sceneElement;
  const frameRef = useRef(frame);
  frameRef.current = frame;
  const contentRef = useRef(content);
  contentRef.current = content;

  useEffect(() => {
    if (preference !== "reference") return;
    setReady(false);
    reportRef.current({
      ...REFERENCE_STATUS,
      detail: "the reference renderer was selected by the view envelope",
    });
  }, [frame.snapshot_id, preference]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || preference === "reference") return;
    let cancelled = false;
    let root: ReturnType<typeof createRoot> | undefined;
    const adapter = new ReactThreeFiberRendererAdapter();
    adapterRef.current = adapter;
    setSubmittedFrame(undefined);
    setReady(false);
    reportRef.current({
      lifecycle: "initializing",
      backend: null,
      renderer_revision: THREE_RENDERER_REVISION,
      degraded: false,
      detail: "initializing the declarative React Three Fiber renderer",
    });
    const unsubscribeStatus = adapter.onStatusChange((status) => {
      if (status.lifecycle !== "failed") return;
      rootRef.current?.unmount();
      rootRef.current = undefined;
      lastFrameRef.current = undefined;
      setSubmittedFrame(undefined);
      setReady(false);
      reportRef.current(status);
    });
    const unsubscribeFrame = adapter.onFrameSubmitted(() => {
      if (cancelled) return;
      const submitted = lastFrameRef.current;
      if (submitted) setSubmittedFrame(Object.freeze({ ...submitted }));
      setReady(true);
      reportRef.current(adapter.status, submitted ?? null);
    });
    const handleContextLoss = (event: Event) => {
      event.preventDefault();
      rootRef.current?.unmount();
      rootRef.current = undefined;
      lastFrameRef.current = undefined;
      setSubmittedFrame(undefined);
      setReady(false);
      reportRef.current({
        lifecycle: "failed",
        backend: "reference",
        renderer_revision: THREE_RENDERER_REVISION,
        degraded: true,
        detail: "graphics context lost; the reference terrain remains active",
      });
    };
    const handleWebGpuDeviceLossQualification = () => {
      if (preference === "webgpu")
        adapter.destroyWebGpuDeviceForQualification();
    };
    canvas.addEventListener("webglcontextlost", handleContextLoss);
    canvas.addEventListener(
      WEBGPU_DEVICE_LOSS_QUALIFICATION_EVENT,
      handleWebGpuDeviceLossQualification,
    );

    void (async () => {
      const status = await adapter.initialize(
        canvas,
        preference === "auto" ? "auto" : preference,
      );
      if (cancelled || status.lifecycle !== "ready" || !adapter.renderer) {
        if (!cancelled) reportRef.current(status);
        return;
      }
      try {
        root = createRoot(canvas);
        const viewport = canvasViewport(contentRef.current);
        await root.configure({
          dpr: viewport.device_pixel_ratio,
          flat: true,
          frameloop: "demand",
          gl: adapter.renderer as unknown as FiberRenderer,
          size: {
            width: viewport.width,
            height: viewport.height,
            left: 0,
            top: 0,
          },
        });
        if (cancelled) {
          root.unmount();
          return;
        }
        rootRef.current = root;
        lastFrameRef.current = frameRef.current;
        root.render(sceneElementRef.current);
        setRootGeneration((generation) => generation + 1);
      } catch (error) {
        setReady(false);
        reportRef.current({
          lifecycle: "failed",
          backend: "reference",
          renderer_revision: THREE_RENDERER_REVISION,
          degraded: true,
          detail: error instanceof Error ? error.message : String(error),
        });
      }
    })();

    return () => {
      cancelled = true;
      canvas.removeEventListener("webglcontextlost", handleContextLoss);
      canvas.removeEventListener(
        WEBGPU_DEVICE_LOSS_QUALIFICATION_EVENT,
        handleWebGpuDeviceLossQualification,
      );
      unsubscribeFrame();
      unsubscribeStatus();
      root?.unmount();
      adapter.dispose();
      if (rootRef.current === root) rootRef.current = undefined;
      if (adapterRef.current === adapter) adapterRef.current = undefined;
      lastFrameRef.current = undefined;
      setSubmittedFrame(undefined);
    };
  }, [preference]);

  useEffect(() => {
    const root = rootRef.current;
    const adapter = adapterRef.current;
    if (!root || !adapter?.renderer || rootGeneration === 0) return;
    void root.configure({
      dpr: viewport.device_pixel_ratio,
      flat: true,
      frameloop: "demand",
      gl: adapter.renderer as unknown as FiberRenderer,
      size: {
        width: viewport.width,
        height: viewport.height,
        left: 0,
        top: 0,
      },
    });
  }, [rootGeneration, viewport.height, viewport.width]);

  useEffect(() => {
    const root = rootRef.current;
    if (!root || rootGeneration === 0) return;
    if (renderFrameInvalidation(lastFrameRef.current, frame).length === 0)
      return;
    lastFrameRef.current = Object.freeze({ ...frame });
    root.render(sceneElement);
  }, [
    frame.camera_revision,
    frame.material_revision,
    frame.render_graph_id,
    frame.snapshot_id,
    rootGeneration,
    sceneElement,
  ]);

  return (
    <canvas
      aria-hidden="true"
      className={joinClassNames(
        className,
        ready && visible ? readyClassName : undefined,
      )}
      data-globe-horizontal-wrap-indexes={
        content.kind === "globe" ? horizontalWrapIndexes.join(",") : undefined
      }
      data-render-kind={content.kind}
      data-rendered-snapshot={submittedFrame?.snapshot_id}
      data-globe-horizontal-wrap-depth={
        content.kind === "globe" ? horizontalWrapDepth.toFixed(3) : undefined
      }
      data-globe-horizontal-wrap-opacity={
        content.kind === "globe" ? horizontalWrapOpacity.toFixed(3) : undefined
      }
      data-globe-atmosphere-opacity={
        content.kind === "globe"
          ? globeAtmosphereOpacity(
              content.view.projection_morph_progress ?? 0,
            ).toFixed(3)
          : undefined
      }
      data-globe-atmosphere-repeat-opacity={
        content.kind === "globe"
          ? globeAtmosphereRepeatOpacity(
              content.view.projection_morph_progress ?? 0,
            ).toFixed(3)
          : undefined
      }
      data-globe-atmosphere-shell-scale={
        content.kind === "globe"
          ? globeAtmosphereShellScale(
              content.view.projection_morph_progress ?? 0,
            ).toFixed(3)
          : undefined
      }
      data-globe-surface-opacity={
        content.kind === "globe"
          ? globeSurfaceOpacity(
              content.view.projection_morph_progress ?? 0,
            ).toFixed(3)
          : undefined
      }
      data-renderer="react-three-fiber"
      ref={canvasRef}
      style={wrappedCanvasStyle}
    />
  );
}

function canvasViewport(content: ExplorerCanvasContent) {
  const horizontalWraps = globeHorizontalWrapIndexes(content).length;
  return boundedViewport(
    {
      width:
        content.kind === "globe"
          ? content.world.width * horizontalWraps
          : content.view.viewport_width,
      height:
        content.kind === "globe"
          ? content.world.height
          : content.view.viewport_height,
      device_pixel_ratio: globalThis.devicePixelRatio ?? 1,
    },
    2,
    horizontalWraps > 1 ? 4_096 : 2_048,
  );
}

function globeHorizontalWrapIndexes(content: ExplorerCanvasContent) {
  return content.kind === "globe"
    ? GLOBE_ATLAS_HORIZONTAL_WRAP_INDEXES
    : ([0] as const);
}

function globeHorizontalWrapOpacity(content: ExplorerCanvasContent) {
  return content.kind === "globe"
    ? globeAtlasRepeatOpacity(content.view.projection_morph_progress ?? 0)
    : 0;
}

function globeHorizontalWrapDepth(content: ExplorerCanvasContent) {
  return content.kind === "globe"
    ? globeAtlasRepeatDepthOffset(
        content.view.projection_morph_progress ?? 0,
        0,
      )
    : 0;
}

function joinClassNames(...values: (string | undefined)[]) {
  return values.filter((value): value is string => Boolean(value)).join(" ");
}
