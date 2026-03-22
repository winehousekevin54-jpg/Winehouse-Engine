import { useEffect, useRef, useCallback } from 'react';
import { renderFrame, resizeViewport, cameraOrbit, cameraZoom, getSceneObjects } from '../bridge/EngineAPI';
import { useEditorStore } from '../store/editor';

export function SceneView() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const isDragging = useRef(false);
  const lastMouse = useRef({ x: 0, y: 0 });
  const rafRef = useRef<number>(0);
  const setEntities = useEditorStore((s) => s.setEntities);
  const engineStatus = useEditorStore((s) => s.engineStatus);

  // Render loop
  useEffect(() => {
    if (engineStatus !== 'running') return;

    let frameCount = 0;
    function loop() {
      renderFrame();
      frameCount++;
      // Refresh scene list every 30 frames (~0.5s)
      if (frameCount % 30 === 0) {
        setEntities(getSceneObjects());
      }
      rafRef.current = requestAnimationFrame(loop);
    }
    rafRef.current = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(rafRef.current);
  }, [engineStatus, setEntities]);

  // Resize observer — account for device pixel ratio (Retina / HiDPI)
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const obs = new ResizeObserver(() => {
      const dpr = window.devicePixelRatio || 1;
      const w = Math.round(canvas.clientWidth * dpr);
      const h = Math.round(canvas.clientHeight * dpr);
      canvas.width = w;
      canvas.height = h;
      resizeViewport(w, h);
    });
    obs.observe(canvas);
    return () => obs.disconnect();
  }, []);

  const onMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button === 0) {
      isDragging.current = true;
      lastMouse.current = { x: e.clientX, y: e.clientY };
    }
  }, []);

  const onMouseMove = useCallback((e: React.MouseEvent) => {
    if (!isDragging.current) return;
    const dx = e.clientX - lastMouse.current.x;
    const dy = e.clientY - lastMouse.current.y;
    lastMouse.current = { x: e.clientX, y: e.clientY };
    // Scale: 0.005 rad per pixel
    cameraOrbit(dx * 0.005, -dy * 0.005);
  }, []);

  const onMouseUp = useCallback(() => {
    isDragging.current = false;
  }, []);

  // Must use a native listener with { passive: false } — React's onWheel is passive
  // and cannot call preventDefault(), causing the browser page to zoom as well.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const handler = (e: WheelEvent) => {
      e.preventDefault();
      cameraZoom(-e.deltaY * 0.001);
    };
    canvas.addEventListener('wheel', handler, { passive: false });
    return () => canvas.removeEventListener('wheel', handler);
  }, []);

  return (
    <div style={{ width: '100%', height: '100%', position: 'relative', background: '#0d0d12' }}>
      <canvas
        id="viewport"
        ref={canvasRef}
        style={{ width: '100%', height: '100%', display: 'block', cursor: isDragging.current ? 'grabbing' : 'grab' }}
        onMouseDown={onMouseDown}
        onMouseMove={onMouseMove}
        onMouseUp={onMouseUp}
        onMouseLeave={onMouseUp}
      />
      {engineStatus === 'loading' && (
        <div style={{
          position: 'absolute', top: '50%', left: '50%',
          transform: 'translate(-50%, -50%)',
          color: '#fbbf24', fontSize: 13,
        }}>
          Initializing WebGPU…
        </div>
      )}
    </div>
  );
}
