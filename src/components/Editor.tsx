import { useCallback, useEffect, useRef, useState } from 'react';
import { api, Shot } from '../api';
import { Dict } from '../i18n';
import {
  IconArrow,
  IconCopy,
  IconCrop,
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

type Tool = 'pen' | 'line' | 'arrow' | 'rect' | 'ellipse' | 'text' | 'pixelate' | 'crop';

interface Shape {
  tool: Exclude<Tool, 'crop'>;
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
      const fontSize = 10 + s.width * 7;
      ctx.font = `600 ${fontSize}px -apple-system, 'Segoe UI', sans-serif`;
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
  const [baseVersion, setBaseVersion] = useState(0);
  const [tool, setTool] = useState<Tool>('arrow');
  const [color, setColor] = useState(COLORS[0]);
  const [width, setWidth] = useState(4);
  const [shapes, setShapes] = useState<Shape[]>([]);
  const [redoStack, setRedoStack] = useState<Shape[]>([]);
  const [draft, setDraft] = useState<Shape | null>(null);
  const [cropDraft, setCropDraft] = useState<Shape | null>(null);
  const [textPos, setTextPos] = useState<{ x: number; y: number } | null>(null);
  const [textValue, setTextValue] = useState('');
  const [name, setName] = useState(shot.name);
  const [tags, setTags] = useState(shot.tags.join(', '));
  const [dirty, setDirty] = useState(false);

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
        setRedoStack([]);
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
    for (const s of shapes) drawShape(ctx, s, base);
    if (draft) drawShape(ctx, draft, base);
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
  }, [baseVersion, shapes, draft, cropDraft]);

  const toCanvasCoords = (e: React.MouseEvent): { x: number; y: number } => {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    return {
      x: ((e.clientX - rect.left) / rect.width) * canvas.width,
      y: ((e.clientY - rect.top) / rect.height) * canvas.height,
    };
  };

  const commitText = useCallback(() => {
    if (textPos && textValue.trim()) {
      setShapes((prev) => [
        ...prev,
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
      setRedoStack([]);
      setDirty(true);
    }
    setTextPos(null);
    setTextValue('');
  }, [textPos, textValue, color, width]);

  const onMouseDown = (e: React.MouseEvent) => {
    if (!baseRef.current) return;
    if (textPos) {
      commitText();
      return;
    }
    const { x, y } = toCanvasCoords(e);
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
    if (!draft && !cropDraft) return;
    const { x, y } = toCanvasCoords(e);
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
    if (draft) {
      const moved =
        Math.abs(draft.x2 - draft.x1) > 2 ||
        Math.abs(draft.y2 - draft.y1) > 2 ||
        (draft.points?.length ?? 0) > 2;
      if (moved) {
        setShapes((prev) => [...prev, draft]);
        setRedoStack([]);
        setDirty(true);
      }
      setDraft(null);
    }
  };

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
    setRedoStack([]);
    setCropDraft(null);
    setDirty(true);
    setBaseVersion((v) => v + 1);
  };

  const undo = () => {
    setShapes((prev) => {
      if (prev.length === 0) return prev;
      const last = prev[prev.length - 1];
      setRedoStack((r) => [...r, last]);
      return prev.slice(0, -1);
    });
  };

  const redo = () => {
    setRedoStack((prev) => {
      if (prev.length === 0) return prev;
      const last = prev[prev.length - 1];
      setShapes((s) => [...s, last]);
      return prev.slice(0, -1);
    });
  };

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
            onClick={() => {
              setTool(tl.key);
              if (tl.key !== 'crop') setCropDraft(null);
            }}
          >
            {tl.icon}
          </button>
        ))}
        <span className="sep" />
        {COLORS.map((c) => (
          <button
            key={c}
            className={`colorbtn ${color === c ? 'active' : ''}`}
            style={{ background: c }}
            onClick={() => setColor(c)}
          />
        ))}
        <span className="sep" />
        {WIDTHS.map((w) => (
          <button
            key={w}
            className={`icon widthbtn ${width === w ? 'active' : ''}`}
            onClick={() => setWidth(w)}
          >
            {w}px
          </button>
        ))}
        <span className="sep" />
        <button className="icon" title={t.undo} onClick={undo} disabled={shapes.length === 0}>
          <IconUndo />
        </button>
        <button className="icon" title={t.redo} onClick={redo} disabled={redoStack.length === 0}>
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
          <div className="note">{t.pixelateNote}</div>
        </div>
      </div>
    </>
  );
}
