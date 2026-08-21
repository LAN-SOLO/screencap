import { useCallback, useEffect, useRef, useState } from 'react';
import { api, Shot } from '../api';
import { Dict } from '../i18n';
import {
  IconArrow,
  IconCopy,
  IconCrop,
  IconCursor,
  IconEllipse,
  IconFolder,
  IconLine,
  IconPen,
  IconPixelate,
  IconRect,
  IconRedo,
  IconStar,
  IconText,
  IconTrash,
  IconUndo,
} from '../icons';

type Tool = 'select' | 'pen' | 'line' | 'arrow' | 'rect' | 'ellipse' | 'text' | 'pixelate' | 'crop';

interface Shape {
  tool: Exclude<Tool, 'crop' | 'select'>;
  color: string;
  width: number;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  points?: { x: number; y: number }[];
  text?: string;
}

const COLORS = ['#f87171', '#fbbf24', '#34d399', '#38bdf8', '#ffffff', '#111827'];
const WIDTHS = [2, 4, 8];

function drawArrowHead(
  ctx: CanvasRenderingContext2D,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  width: number
) {
  const angle = Math.atan2(y2 - y1, x2 - x1);
  const len = Math.max(12, width * 4);
  ctx.beginPath();
  ctx.moveTo(x2, y2);
  ctx.lineTo(x2 - len * Math.cos(angle - Math.PI / 7), y2 - len * Math.sin(angle - Math.PI / 7));
  ctx.moveTo(x2, y2);
  ctx.lineTo(x2 - len * Math.cos(angle + Math.PI / 7), y2 - len * Math.sin(angle + Math.PI / 7));
  ctx.stroke();
}

function textFont(s: Shape): { font: string; fontSize: number } {
  const fontSize = 10 + s.width * 7;
  return { font: `600 ${fontSize}px -apple-system, 'Segoe UI', sans-serif`, fontSize };
}

function drawShape(ctx: CanvasRenderingContext2D, s: Shape, base: HTMLCanvasElement) {
  ctx.save();
  ctx.strokeStyle = s.color;
  ctx.fillStyle = s.color;
  ctx.lineWidth = s.width;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  switch (s.tool) {
    case 'pen': {
      if (!s.points || s.points.length === 0) break;
      ctx.beginPath();
      ctx.moveTo(s.points[0].x, s.points[0].y);
      for (const p of s.points) ctx.lineTo(p.x, p.y);
      ctx.stroke();
      break;
    }
    case 'line': {
      ctx.beginPath();
      ctx.moveTo(s.x1, s.y1);
      ctx.lineTo(s.x2, s.y2);
      ctx.stroke();
      break;
    }
    case 'arrow': {
      ctx.beginPath();
      ctx.moveTo(s.x1, s.y1);
      ctx.lineTo(s.x2, s.y2);
      ctx.stroke();
      drawArrowHead(ctx, s.x1, s.y1, s.x2, s.y2, s.width);
      break;
    }
    case 'rect': {
      ctx.strokeRect(
        Math.min(s.x1, s.x2),
        Math.min(s.y1, s.y2),
        Math.abs(s.x2 - s.x1),
        Math.abs(s.y2 - s.y1)
      );
      break;
    }
    case 'ellipse': {
      ctx.beginPath();
      ctx.ellipse(
        (s.x1 + s.x2) / 2,
        (s.y1 + s.y2) / 2,
        Math.abs(s.x2 - s.x1) / 2,
        Math.abs(s.y2 - s.y1) / 2,
        0,
        0,
        Math.PI * 2
      );
      ctx.stroke();
      break;
    }
    case 'text': {
      const { font, fontSize } = textFont(s);
      ctx.font = font;
      ctx.textBaseline = 'top';
      const lines = (s.text ?? '').split('\n');
      lines.forEach((line, i) => {
        ctx.fillText(line, s.x1, s.y1 + i * fontSize * 1.25);
      });
      break;
    }
    case 'pixelate': {
      const x = Math.round(Math.min(s.x1, s.x2));
      const y = Math.round(Math.min(s.y1, s.y2));
      const w = Math.round(Math.abs(s.x2 - s.x1));
      const h = Math.round(Math.abs(s.y2 - s.y1));
      if (w < 2 || h < 2) break;
      const block = 14;
      const sw = Math.max(1, Math.round(w / block));
      const sh = Math.max(1, Math.round(h / block));
      const off = document.createElement('canvas');
      off.width = sw;
      off.height = sh;
      const octx = off.getContext('2d')!;
      octx.imageSmoothingEnabled = true;
      octx.drawImage(base, x, y, w, h, 0, 0, sw, sh);
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(off, 0, 0, sw, sh, x, y, w, h);
      ctx.imageSmoothingEnabled = true;
      break;
    }
  }
  ctx.restore();
}

// ---------- selection helpers ----------

function shapeBBox(s: Shape, ctx: CanvasRenderingContext2D): { x: number; y: number; w: number; h: number } {
  if (s.tool === 'pen' && s.points && s.points.length > 0) {
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const p of s.points) {
      minX = Math.min(minX, p.x);
      minY = Math.min(minY, p.y);
      maxX = Math.max(maxX, p.x);
      maxY = Math.max(maxY, p.y);
    }
    return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
  }
  if (s.tool === 'text') {
    const { font, fontSize } = textFont(s);
    ctx.save();
    ctx.font = font;
    const lines = (s.text ?? '').split('\n');
    let w = 0;
    for (const line of lines) w = Math.max(w, ctx.measureText(line).width);
    ctx.restore();
    return { x: s.x1, y: s.y1, w, h: lines.length * fontSize * 1.25 };
  }
  return {
    x: Math.min(s.x1, s.x2),
    y: Math.min(s.y1, s.y2),
    w: Math.abs(s.x2 - s.x1),
    h: Math.abs(s.y2 - s.y1),
  };
}

function distToSegment(px: number, py: number, x1: number, y1: number, x2: number, y2: number) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const lenSq = dx * dx + dy * dy;
  const t = lenSq === 0 ? 0 : Math.max(0, Math.min(1, ((px - x1) * dx + (py - y1) * dy) / lenSq));
  const cx = x1 + t * dx;
  const cy = y1 + t * dy;
  return Math.hypot(px - cx, py - cy);
}

function hitShape(s: Shape, x: number, y: number, tol: number, ctx: CanvasRenderingContext2D): boolean {
  const reach = tol + s.width / 2;
  switch (s.tool) {
    case 'line':
    case 'arrow':
      return distToSegment(x, y, s.x1, s.y1, s.x2, s.y2) <= reach;
    case 'pen': {
      const pts = s.points ?? [];
      for (let i = 1; i < pts.length; i++) {
        if (distToSegment(x, y, pts[i - 1].x, pts[i - 1].y, pts[i].x, pts[i].y) <= reach) return true;
      }
      return false;
    }
    default: {
      const b = shapeBBox(s, ctx);
      return x >= b.x - tol && x <= b.x + b.w + tol && y >= b.y - tol && y <= b.y + b.h + tol;
    }
  }
}

function moveShape(s: Shape, dx: number, dy: number): Shape {
  return {
    ...s,
    x1: s.x1 + dx,
    y1: s.y1 + dy,
    x2: s.x2 + dx,
    y2: s.y2 + dy,
    points: s.points?.map((p) => ({ x: p.x + dx, y: p.y + dy })),
  };
}

/** Boxy shapes get x1/y1 = top-left so handle indices stay stable while resizing. */
function normalizeShape(s: Shape): Shape {
  if (s.tool === 'line' || s.tool === 'arrow' || s.tool === 'pen' || s.tool === 'text') return s;
  return {
    ...s,
    x1: Math.min(s.x1, s.x2),
    y1: Math.min(s.y1, s.y2),
    x2: Math.max(s.x1, s.x2),
    y2: Math.max(s.y1, s.y2),
  };
}

/** Resize handles: endpoints for lines/arrows, corners for boxy shapes. */
function shapeHandles(s: Shape): { x: number; y: number }[] {
  if (s.tool === 'line' || s.tool === 'arrow') {
    return [
      { x: s.x1, y: s.y1 },
      { x: s.x2, y: s.y2 },
    ];
  }
  if (s.tool === 'pen' || s.tool === 'text') return [];
  return [
    { x: s.x1, y: s.y1 },
    { x: s.x2, y: s.y1 },
    { x: s.x1, y: s.y2 },
    { x: s.x2, y: s.y2 },
  ];
}

function applyHandle(s: Shape, handle: number, x: number, y: number): Shape {
  if (s.tool === 'line' || s.tool === 'arrow') {
    return handle === 0 ? { ...s, x1: x, y1: y } : { ...s, x2: x, y2: y };
  }
  switch (handle) {
    case 0:
      return { ...s, x1: x, y1: y };
    case 1:
      return { ...s, x2: x, y1: y };
    case 2:
      return { ...s, x1: x, y2: y };
    default:
      return { ...s, x2: x, y2: y };
  }
}

type Drag =
  | { kind: 'move'; idx: number; startX: number; startY: number; orig: Shape; origAll: Shape[]; moved: boolean }
  | { kind: 'handle'; idx: number; handle: number; orig: Shape; origAll: Shape[]; moved: boolean };

export function Editor({
  shot,
  t,
  onBack,
  onShotChanged,
  showToast,
}: {
  shot: Shot;
  t: Dict;
  onBack: () => void;
  onShotChanged: (s: Shot | null) => void;
  showToast: (msg: string) => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const baseRef = useRef<HTMLCanvasElement | null>(null);
  const dragRef = useRef<Drag | null>(null);
  const [baseVersion, setBaseVersion] = useState(0);
  const [tool, setToolState] = useState<Tool>('arrow');
  const [color, setColor] = useState(COLORS[0]);
  const [width, setWidth] = useState(4);
  const [shapes, setShapes] = useState<Shape[]>([]);
  const [past, setPast] = useState<Shape[][]>([]);
  const [future, setFuture] = useState<Shape[][]>([]);
  const [selectedIdx, setSelectedIdx] = useState<number | null>(null);
  const [draft, setDraft] = useState<Shape | null>(null);
  const [cropDraft, setCropDraft] = useState<Shape | null>(null);
  const [textPos, setTextPos] = useState<{ x: number; y: number } | null>(null);
  const [textValue, setTextValue] = useState('');
  const [textEditIdx, setTextEditIdx] = useState<number | null>(null);
  const [name, setName] = useState(shot.name);
  const [tags, setTags] = useState(shot.tags.join(', '));
  const [dirty, setDirty] = useState(false);

  const setTool = (next: Tool) => {
    setToolState(next);
    if (next !== 'crop') setCropDraft(null);
    if (next !== 'select') setSelectedIdx(null);
  };

  // every mutation goes through commitShapes so undo/redo covers add,
  // move, resize, restyle, text edits and deletes alike
  const pushHistory = useCallback((prev: Shape[]) => {
    setPast((p) => [...p, prev]);
    setFuture([]);
    setDirty(true);
  }, []);

  const commitShapes = useCallback(
    (next: Shape[]) => {
      pushHistory(shapes);
      setShapes(next);
    },
    [shapes, pushHistory]
  );

  // load full-res image into an offscreen base canvas
  useEffect(() => {
    let alive = true;
    api.getImage(shot.id).then((dataUrl) => {
      const img = new Image();
      img.onload = () => {
        if (!alive) return;
        const c = document.createElement('canvas');
        c.width = img.naturalWidth;
        c.height = img.naturalHeight;
        c.getContext('2d')!.drawImage(img, 0, 0);
        baseRef.current = c;
        setShapes([]);
        setPast([]);
        setFuture([]);
        setSelectedIdx(null);
        setDirty(false);
        setBaseVersion((v) => v + 1);
      };
      img.src = dataUrl;
    });
    return () => {
      alive = false;
    };
  }, [shot.id]);

  // redraw
  useEffect(() => {
    const canvas = canvasRef.current;
    const base = baseRef.current;
    if (!canvas || !base) return;
    canvas.width = base.width;
    canvas.height = base.height;
    const ctx = canvas.getContext('2d')!;
    ctx.drawImage(base, 0, 0);
    shapes.forEach((s, i) => {
      if (i === textEditIdx) return; // being edited in the overlay
      drawShape(ctx, s, base);
    });
    if (draft) drawShape(ctx, draft, base);
    if (selectedIdx != null && shapes[selectedIdx] && textEditIdx == null) {
      const s = shapes[selectedIdx];
      const b = shapeBBox(s, ctx);
      const lw = Math.max(1.5, canvas.width / 800);
      const pad = lw * 3;
      ctx.save();
      ctx.strokeStyle = '#38bdf8';
      ctx.lineWidth = lw;
      ctx.setLineDash([6 * lw, 4 * lw]);
      ctx.strokeRect(b.x - pad, b.y - pad, b.w + pad * 2, b.h + pad * 2);
      ctx.setLineDash([]);
      const r = Math.max(4, canvas.width / 220);
      for (const h of shapeHandles(s)) {
        ctx.fillStyle = '#38bdf8';
        ctx.strokeStyle = '#ffffff';
        ctx.beginPath();
        ctx.rect(h.x - r, h.y - r, r * 2, r * 2);
        ctx.fill();
        ctx.stroke();
      }
      ctx.restore();
    }
    if (cropDraft) {
      const x = Math.min(cropDraft.x1, cropDraft.x2);
      const y = Math.min(cropDraft.y1, cropDraft.y2);
      const w = Math.abs(cropDraft.x2 - cropDraft.x1);
      const h = Math.abs(cropDraft.y2 - cropDraft.y1);
      ctx.save();
      ctx.fillStyle = 'rgba(4, 8, 16, 0.6)';
      ctx.beginPath();
      ctx.rect(0, 0, canvas.width, canvas.height);
      ctx.rect(x, y, w, h);
      ctx.fill('evenodd');
      ctx.strokeStyle = '#38bdf8';
      ctx.lineWidth = Math.max(2, canvas.width / 600);
      ctx.setLineDash([8, 6]);
      ctx.strokeRect(x, y, w, h);
      ctx.restore();
    }
  }, [baseVersion, shapes, draft, cropDraft, selectedIdx, textEditIdx]);

  const toCanvasCoords = (e: React.MouseEvent): { x: number; y: number } => {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    return {
      x: ((e.clientX - rect.left) / rect.width) * canvas.width,
      y: ((e.clientY - rect.top) / rect.height) * canvas.height,
    };
  };

  const hitTolerance = () => Math.max(6, (canvasRef.current?.width ?? 800) / 150);

  const commitText = useCallback(() => {
    if (textEditIdx != null && shapes[textEditIdx]) {
      const next = [...shapes];
      if (textValue.trim()) next[textEditIdx] = { ...next[textEditIdx], text: textValue };
      else next.splice(textEditIdx, 1);
      commitShapes(next);
      if (!textValue.trim()) setSelectedIdx(null);
    } else if (textPos && textValue.trim()) {
      commitShapes([
        ...shapes,
        {
          tool: 'text',
          color,
          width,
          x1: textPos.x,
          y1: textPos.y,
          x2: textPos.x,
          y2: textPos.y,
          text: textValue,
        },
      ]);
    }
    setTextPos(null);
    setTextValue('');
    setTextEditIdx(null);
  }, [textPos, textValue, textEditIdx, shapes, color, width, commitShapes]);

  const onMouseDown = (e: React.MouseEvent) => {
    if (!baseRef.current) return;
    if (textPos) {
      commitText();
      return;
    }
    const { x, y } = toCanvasCoords(e);

    if (tool === 'select') {
      const ctx = canvasRef.current!.getContext('2d')!;
      const tol = hitTolerance();
      if (selectedIdx != null && shapes[selectedIdx]) {
        const hs = shapeHandles(shapes[selectedIdx]);
        const hi = hs.findIndex((h) => Math.abs(h.x - x) <= tol * 1.4 && Math.abs(h.y - y) <= tol * 1.4);
        if (hi >= 0) {
          const norm = normalizeShape(shapes[selectedIdx]);
          if (norm !== shapes[selectedIdx])
            setShapes(shapes.map((s, i) => (i === selectedIdx ? norm : s)));
          dragRef.current = {
            kind: 'handle',
            idx: selectedIdx,
            handle: hi,
            orig: norm,
            origAll: shapes,
            moved: false,
          };
          return;
        }
      }
      for (let i = shapes.length - 1; i >= 0; i--) {
        if (hitShape(shapes[i], x, y, tol, ctx)) {
          setSelectedIdx(i);
          dragRef.current = {
            kind: 'move',
            idx: i,
            startX: x,
            startY: y,
            orig: shapes[i],
            origAll: shapes,
            moved: false,
          };
          return;
        }
      }
      setSelectedIdx(null);
      return;
    }

    if (tool === 'text') {
      setTextPos({ x, y });
      return;
    }
    const shape: Shape = {
      tool: tool === 'crop' ? 'rect' : tool,
      color,
      width,
      x1: x,
      y1: y,
      x2: x,
      y2: y,
      points: tool === 'pen' ? [{ x, y }] : undefined,
    };
    if (tool === 'crop') {
      setCropDraft(shape);
    } else {
      setDraft(shape);
    }
  };

  const onMouseMove = (e: React.MouseEvent) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const { x, y } = toCanvasCoords(e);

    if (tool === 'select') {
      const d = dragRef.current;
      if (d) {
        let next: Shape;
        if (d.kind === 'move') {
          const dx = x - d.startX;
          const dy = y - d.startY;
          if (Math.abs(dx) > 0.5 || Math.abs(dy) > 0.5) d.moved = true;
          next = moveShape(d.orig, dx, dy);
        } else {
          d.moved = true;
          next = applyHandle(d.orig, d.handle, x, y);
        }
        setShapes((prev) => prev.map((s, i) => (i === d.idx ? next : s)));
        return;
      }
      // hover feedback
      const ctx = canvas.getContext('2d')!;
      const tol = hitTolerance();
      let cursor = 'default';
      if (selectedIdx != null && shapes[selectedIdx]) {
        const hs = shapeHandles(shapes[selectedIdx]);
        if (hs.some((h) => Math.abs(h.x - x) <= tol * 1.4 && Math.abs(h.y - y) <= tol * 1.4))
          cursor = 'nwse-resize';
      }
      if (cursor === 'default') {
        for (let i = shapes.length - 1; i >= 0; i--) {
          if (hitShape(shapes[i], x, y, tol, ctx)) {
            cursor = 'move';
            break;
          }
        }
      }
      canvas.style.cursor = cursor;
      return;
    }
    canvas.style.cursor = 'crosshair';

    if (!draft && !cropDraft) return;
    if (cropDraft) {
      setCropDraft({ ...cropDraft, x2: x, y2: y });
      return;
    }
    if (draft) {
      const next: Shape = { ...draft, x2: x, y2: y };
      if (draft.tool === 'pen') next.points = [...(draft.points ?? []), { x, y }];
      setDraft(next);
    }
  };

  const onMouseUp = () => {
    const d = dragRef.current;
    if (d) {
      if (d.moved) pushHistory(d.origAll);
      dragRef.current = null;
      return;
    }
    if (draft) {
      const moved =
        Math.abs(draft.x2 - draft.x1) > 2 ||
        Math.abs(draft.y2 - draft.y1) > 2 ||
        (draft.points?.length ?? 0) > 2;
      if (moved) commitShapes([...shapes, draft]);
      setDraft(null);
    }
  };

  const onDoubleClick = (e: React.MouseEvent) => {
    if (tool !== 'select' || textPos) return;
    const { x, y } = toCanvasCoords(e);
    const ctx = canvasRef.current!.getContext('2d')!;
    const tol = hitTolerance();
    for (let i = shapes.length - 1; i >= 0; i--) {
      const s = shapes[i];
      if (s.tool === 'text' && hitShape(s, x, y, tol, ctx)) {
        setSelectedIdx(i);
        setTextEditIdx(i);
        setTextPos({ x: s.x1, y: s.y1 });
        setTextValue(s.text ?? '');
        return;
      }
    }
  };

  // Delete removes, arrows nudge (Shift = 10 px), Escape deselects
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) return;
      if (selectedIdx == null || !shapes[selectedIdx]) return;
      if (e.key === 'Backspace' || e.key === 'Delete') {
        e.preventDefault();
        commitShapes(shapes.filter((_, i) => i !== selectedIdx));
        setSelectedIdx(null);
      } else if (e.key === 'Escape') {
        setSelectedIdx(null);
      } else if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key)) {
        e.preventDefault();
        const step = e.shiftKey ? 10 : 1;
        const dx = e.key === 'ArrowLeft' ? -step : e.key === 'ArrowRight' ? step : 0;
        const dy = e.key === 'ArrowUp' ? -step : e.key === 'ArrowDown' ? step : 0;
        commitShapes(shapes.map((s, i) => (i === selectedIdx ? moveShape(s, dx, dy) : s)));
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [selectedIdx, shapes, commitShapes]);

  const bakeToCanvas = (): HTMLCanvasElement => {
    const base = baseRef.current!;
    const out = document.createElement('canvas');
    out.width = base.width;
    out.height = base.height;
    const ctx = out.getContext('2d')!;
    ctx.drawImage(base, 0, 0);
    for (const s of shapes) drawShape(ctx, s, base);
    return out;
  };

  const applyCrop = () => {
    if (!cropDraft || !baseRef.current) return;
    const x = Math.round(Math.min(cropDraft.x1, cropDraft.x2));
    const y = Math.round(Math.min(cropDraft.y1, cropDraft.y2));
    const w = Math.round(Math.abs(cropDraft.x2 - cropDraft.x1));
    const h = Math.round(Math.abs(cropDraft.y2 - cropDraft.y1));
    if (w < 4 || h < 4) {
      setCropDraft(null);
      return;
    }
    const baked = bakeToCanvas();
    const next = document.createElement('canvas');
    next.width = w;
    next.height = h;
    next.getContext('2d')!.drawImage(baked, x, y, w, h, 0, 0, w, h);
    baseRef.current = next;
    setShapes([]);
    setPast([]);
    setFuture([]);
    setSelectedIdx(null);
    setCropDraft(null);
    setDirty(true);
    setBaseVersion((v) => v + 1);
  };

  const undo = () => {
    if (past.length === 0) return;
    const prev = past[past.length - 1];
    setPast(past.slice(0, -1));
    setFuture([...future, shapes]);
    setShapes(prev);
    setSelectedIdx(null);
    setDirty(true);
  };

  const redo = () => {
    if (future.length === 0) return;
    const next = future[future.length - 1];
    setFuture(future.slice(0, -1));
    setPast([...past, shapes]);
    setShapes(next);
    setSelectedIdx(null);
    setDirty(true);
  };

  // color/width apply to the selected object; otherwise they set the default
  const applyColor = (c: string) => {
    setColor(c);
    if (selectedIdx != null && shapes[selectedIdx] && shapes[selectedIdx].color !== c) {
      commitShapes(shapes.map((s, i) => (i === selectedIdx ? { ...s, color: c } : s)));
    }
  };

  const applyWidth = (w: number) => {
    setWidth(w);
    if (selectedIdx != null && shapes[selectedIdx] && shapes[selectedIdx].width !== w) {
      commitShapes(shapes.map((s, i) => (i === selectedIdx ? { ...s, width: w } : s)));
    }
  };

  const activeColor = selectedIdx != null && shapes[selectedIdx] ? shapes[selectedIdx].color : color;
  const activeWidth = selectedIdx != null && shapes[selectedIdx] ? shapes[selectedIdx].width : width;

  const save = (mode: 'overwrite' | 'copy') => {
    const dataUrl = bakeToCanvas().toDataURL('image/png');
    const b64 = dataUrl.slice(dataUrl.indexOf(',') + 1);
    api
      .saveEdit(shot.id, b64, mode)
      .then((updated) => {
        showToast(t.saved);
        setDirty(false);
        onShotChanged(updated);
      })
      .catch((e) => showToast(String(e)));
  };

  // text input overlay position in display coordinates
  let textInputStyle: React.CSSProperties | undefined;
  if (textPos && canvasRef.current && wrapRef.current) {
    const canvas = canvasRef.current;
    const crect = canvas.getBoundingClientRect();
    const wrect = wrapRef.current.getBoundingClientRect();
    textInputStyle = {
      left: crect.left - wrect.left + (textPos.x / canvas.width) * crect.width,
      top: crect.top - wrect.top + (textPos.y / canvas.height) * crect.height,
    };
  }

  const tools: { key: Tool; icon: JSX.Element; label: string }[] = [
    { key: 'select', icon: <IconCursor />, label: t.toolSelect },
    { key: 'pen', icon: <IconPen />, label: t.toolPen },
    { key: 'line', icon: <IconLine />, label: t.toolLine },
    { key: 'arrow', icon: <IconArrow />, label: t.toolArrow },
    { key: 'rect', icon: <IconRect />, label: t.toolRect },
    { key: 'ellipse', icon: <IconEllipse />, label: t.toolEllipse },
    { key: 'text', icon: <IconText />, label: t.toolText },
    { key: 'pixelate', icon: <IconPixelate />, label: t.toolPixelate },
    { key: 'crop', icon: <IconCrop />, label: t.toolCrop },
  ];

  const fmtSize = (b: number) =>
    b > 1024 * 1024 ? `${(b / 1024 / 1024).toFixed(1)} MB` : `${Math.round(b / 1024)} KB`;

  return (
    <>
      <div className="toolbar2">
        <button onClick={onBack}>← {t.back}</button>
        <span className="sep" />
        {tools.map((tl) => (
          <button
            key={tl.key}
            className={`icon ${tool === tl.key ? 'active' : ''}`}
            title={tl.label}
            onClick={() => setTool(tl.key)}
          >
            {tl.icon}
          </button>
        ))}
        <span className="sep" />
        {COLORS.map((c) => (
          <button
            key={c}
            className={`colorbtn ${activeColor === c ? 'active' : ''}`}
            style={{ background: c }}
            onClick={() => applyColor(c)}
          />
        ))}
        <span className="sep" />
        {WIDTHS.map((w) => (
          <button
            key={w}
            className={`icon widthbtn ${activeWidth === w ? 'active' : ''}`}
            onClick={() => applyWidth(w)}
          >
            {w}px
          </button>
        ))}
        <span className="sep" />
        <button className="icon" title={t.undo} onClick={undo} disabled={past.length === 0}>
          <IconUndo />
        </button>
        <button className="icon" title={t.redo} onClick={redo} disabled={future.length === 0}>
          <IconRedo />
        </button>
        {cropDraft && (
          <>
            <span className="sep" />
            <button className="primary" onClick={applyCrop}>
              {t.applyCrop}
            </button>
            <button onClick={() => setCropDraft(null)}>{t.cancelCrop}</button>
          </>
        )}
        <span style={{ flex: 1 }} />
        <button onClick={() => save('copy')} disabled={!dirty}>
          {t.saveCopy}
        </button>
        <button className="primary" onClick={() => save('overwrite')} disabled={!dirty}>
          {t.saveOverwrite}
        </button>
      </div>
      <div className="editor">
        <div className="canvasarea" ref={wrapRef}>
          <canvas
            ref={canvasRef}
            onMouseDown={onMouseDown}
            onMouseMove={onMouseMove}
            onMouseUp={onMouseUp}
            onMouseLeave={onMouseUp}
            onDoubleClick={onDoubleClick}
          />
          {textPos && textInputStyle && (
            <textarea
              className="textinput-float"
              style={textInputStyle}
              autoFocus
              rows={1}
              placeholder={t.textPlaceholder}
              value={textValue}
              onChange={(e) => setTextValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  commitText();
                }
                if (e.key === 'Escape') {
                  setTextPos(null);
                  setTextValue('');
                  setTextEditIdx(null);
                }
              }}
            />
          )}
        </div>
        <div className="sidepanel">
          <label className="field">
            <span>{t.name}</span>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              onBlur={() => {
                if (name.trim() && name !== shot.name) {
                  api.renameShot(shot.id, name).catch(() => {});
                }
              }}
            />
          </label>
          <label className="field">
            <span>{t.tags}</span>
            <input
              type="text"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              onBlur={() => {
                api
                  .setTags(
                    shot.id,
                    tags.split(',').map((x) => x.trim()).filter(Boolean)
                  )
                  .catch(() => {});
              }}
            />
          </label>
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            <button
              className={shot.favorite ? 'active' : ''}
              onClick={() => {
                api.setFavorite(shot.id, !shot.favorite);
                onShotChanged({ ...shot, favorite: !shot.favorite });
              }}
            >
              <IconStar filled={shot.favorite} /> {t.favorite}
            </button>
            <button onClick={() => api.copyShot(shot.id).then(() => showToast(t.copied))}>
              <IconCopy /> {t.copy}
            </button>
            <button onClick={() => api.revealShot(shot.id)}>
              <IconFolder /> {t.reveal}
            </button>
            <button
              className="danger"
              onClick={() => {
                if (window.confirm(t.deleteConfirm)) {
                  api.deleteShot(shot.id).then(() => onShotChanged(null));
                }
              }}
            >
              <IconTrash /> {t.delete}
            </button>
          </div>
          <div className="metagrid">
            <span className="k">px</span>
            <span>
              {shot.width} × {shot.height}
            </span>
            <span className="k">file</span>
            <span style={{ wordBreak: 'break-all' }}>{shot.file}</span>
            <span className="k">size</span>
            <span>{fmtSize(shot.sizeBytes)}</span>
            <span className="k">date</span>
            <span>{new Date(shot.capturedAt).toLocaleString()}</span>
          </div>
          <div className="note">{t.selectHint}</div>
          <div className="note">{t.pixelateNote}</div>
        </div>
      </div>
    </>
  );
}
