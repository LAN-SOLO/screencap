import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Auswahl-Overlay für die Ausschnitt-Aufnahme unter Windows: eigenes,
// transparentes Vollbild-Fenster („region“), das den gezogenen Bereich in
// physischen Pixeln (relativ zum Bildschirm) an `region_result` meldet.
// Esc oder ein Klick ohne Ziehen bricht ab. macOS nutzt `screencapture -i`.

interface Pt {
  x: number;
  y: number;
}

function send(sel: { x: number; y: number; w: number; h: number } | null) {
  invoke('region_result', { sel }).catch(() => {});
}

export function RegionPicker() {
  const [start, setStart] = useState<Pt | null>(null);
  const [cur, setCur] = useState<Pt | null>(null);
  const [done, setDone] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setDone(true);
        send(null);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const rect =
    start && cur
      ? {
          x: Math.min(start.x, cur.x),
          y: Math.min(start.y, cur.y),
          w: Math.abs(cur.x - start.x),
          h: Math.abs(cur.y - start.y),
        }
      : null;

  const finish = () => {
    if (done) return;
    if (!rect || rect.w < 2 || rect.h < 2) {
      setDone(true);
      send(null);
      return;
    }
    const dpr = window.devicePixelRatio || 1;
    setDone(true);
    send({
      x: Math.max(0, Math.round(rect.x * dpr)),
      y: Math.max(0, Math.round(rect.y * dpr)),
      w: Math.max(1, Math.round(rect.w * dpr)),
      h: Math.max(1, Math.round(rect.h * dpr)),
    });
  };

  return (
    <div
      className="region-picker"
      onMouseDown={(e) => {
        if (e.button !== 0) return;
        setStart({ x: e.clientX, y: e.clientY });
        setCur({ x: e.clientX, y: e.clientY });
      }}
      onMouseMove={(e) => {
        if (start) setCur({ x: e.clientX, y: e.clientY });
      }}
      onMouseUp={finish}
      onContextMenu={(e) => {
        e.preventDefault();
        setDone(true);
        send(null);
      }}
    >
      {rect ? (
        <>
          <div className="region-sel" style={{ left: rect.x, top: rect.y, width: rect.w, height: rect.h }} />
          <div
            className="region-size"
            style={{ left: rect.x, top: rect.y + rect.h + 6 > window.innerHeight - 24 ? rect.y - 24 : rect.y + rect.h + 6 }}
          >
            {Math.round(rect.w * (window.devicePixelRatio || 1))} × {Math.round(rect.h * (window.devicePixelRatio || 1))} px
          </div>
        </>
      ) : (
        <div className="region-hint">// Bereich ziehen · Esc = abbrechen &nbsp;|&nbsp; drag a region · Esc = cancel</div>
      )}
    </div>
  );
}
