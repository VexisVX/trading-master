import { useMemo, useState } from "react";
import { EquitySnapshotRow, fmtMoney, parseDec } from "../../lib/equityApi";

interface Props {
  snapshots: EquitySnapshotRow[]; // newest-first
  maxPoints?: number;
}

interface Point {
  t: number;         // timestamp (ms)
  equity: number;
  hwm: number;
  floor: number | null; // equity_floor if available
  label: string;
}

export function EquityCurveChart({ snapshots, maxPoints = 30 }: Props) {
  const [hover, setHover] = useState<{ x: number; p: Point } | null>(null);

  const points = useMemo<Point[]>(() => {
    const ordered = [...snapshots]
      .slice(0, maxPoints)
      .reverse(); // oldest first for chart

    return ordered.map((s) => ({
      t: new Date(s.taken_at).getTime(),
      equity: parseDec(s.equity) ?? 0,
      hwm: parseDec(s.high_watermark) ?? 0,
      floor: parseDec(s.equity_floor ?? undefined) ?? null,
      label: new Date(s.taken_at).toLocaleString(),
    }));
  }, [snapshots, maxPoints]);

  if (points.length < 2) {
    return (
      <div className="ee-curve-empty">
        <div className="ee-curve-empty__icon">📈</div>
        <div className="ee-curve-empty__text">
          {points.length === 0
            ? "No snapshots yet — click ⟳ Refresh to create one."
            : "Need at least 2 snapshots to draw the curve. Refresh after a few trades."}
        </div>
      </div>
    );
  }

  // ---- Compute bounds ----
  const ys: number[] = [];
  for (const p of points) {
    ys.push(p.equity, p.hwm);
    if (p.floor !== null) ys.push(p.floor);
  }
  const minY = Math.min(...ys);
  const maxY = Math.max(...ys);
  const padding = (maxY - minY) * 0.1 || 1;
  const yMin = minY - padding;
  const yMax = maxY + padding;

  // ---- Layout ----
  const W = 700;
  const H = 240;
  const PADDING_L = 60;
  const PADDING_R = 20;
  const PADDING_T = 20;
  const PADDING_B = 30;
  const chartW = W - PADDING_L - PADDING_R;
  const chartH = H - PADDING_T - PADDING_B;

  const xFor = (i: number) =>
    PADDING_L + (i / (points.length - 1)) * chartW;
  const yFor = (v: number) =>
    PADDING_T + chartH - ((v - yMin) / (yMax - yMin)) * chartH;

  // ---- Build polyline strings ----
  const equityPath = points
    .map((p, i) => `${xFor(i)},${yFor(p.equity)}`)
    .join(" ");
  const hwmPath = points.map((p, i) => `${xFor(i)},${yFor(p.hwm)}`).join(" ");

  const floorPoints = points
    .map((p, i) => (p.floor !== null ? { x: xFor(i), y: yFor(p.floor) } : null))
    .filter((x): x is { x: number; y: number } => x !== null);
  const floorPath = floorPoints.map((p) => `${p.x},${p.y}`).join(" ");

  // ---- Y-axis ticks (4) ----
  const yTicks: number[] = [];
  for (let i = 0; i <= 4; i++) {
    yTicks.push(yMin + ((yMax - yMin) * i) / 4);
  }

  function handleMouseMove(e: React.MouseEvent<SVGSVGElement>) {
    const rect = e.currentTarget.getBoundingClientRect();
    const scaleX = W / rect.width;
    const mouseX = (e.clientX - rect.left) * scaleX;
    const xRel = mouseX - PADDING_L;
    if (xRel < 0 || xRel > chartW) {
      setHover(null);
      return;
    }
    const idx = Math.round((xRel / chartW) * (points.length - 1));
    const clamped = Math.max(0, Math.min(points.length - 1, idx));
    setHover({ x: xFor(clamped), p: points[clamped] });
  }

  return (
    <div className="ee-curve">
      <div className="ee-curve__legend">
        <span className="ee-legend">
          <span className="ee-legend__line" style={{ background: "#2dd4bf" }} />
          Equity
        </span>
        <span className="ee-legend">
          <span
            className="ee-legend__line"
            style={{ background: "#fbbf24", borderTop: "1px dashed" }}
          />
          High Watermark
        </span>
        {floorPoints.length > 0 && (
          <span className="ee-legend">
            <span
              className="ee-legend__line"
              style={{ background: "#ff6b6b", borderTop: "1px dashed" }}
            />
            Equity Floor
          </span>
        )}
      </div>

      <svg
        viewBox={`0 0 ${W} ${H}`}
        className="ee-curve__svg"
        onMouseMove={handleMouseMove}
        onMouseLeave={() => setHover(null)}
      >
        {/* Y grid + labels */}
        {yTicks.map((v, i) => (
          <g key={i}>
            <line
              x1={PADDING_L}
              x2={W - PADDING_R}
              y1={yFor(v)}
              y2={yFor(v)}
              stroke="rgba(255,255,255,0.06)"
              strokeWidth={1}
            />
            <text
              x={PADDING_L - 8}
              y={yFor(v)}
              textAnchor="end"
              dominantBaseline="middle"
              fontSize={10}
              fill="#888"
            >
              {fmtMoney(v)}
            </text>
          </g>
        ))}

        {/* Floor (dashed red) */}
        {floorPoints.length > 1 && (
          <polyline
            points={floorPath}
            fill="none"
            stroke="#ff6b6b"
            strokeWidth={1.5}
            strokeDasharray="4 3"
          />
        )}

        {/* HWM (dashed yellow) */}
        <polyline
          points={hwmPath}
          fill="none"
          stroke="#fbbf24"
          strokeWidth={1.5}
          strokeDasharray="4 3"
        />

        {/* Equity (solid teal) */}
        <polyline
          points={equityPath}
          fill="none"
          stroke="#2dd4bf"
          strokeWidth={2.2}
        />

        {/* Equity dots */}
        {points.map((p, i) => (
          <circle
            key={i}
            cx={xFor(i)}
            cy={yFor(p.equity)}
            r={3}
            fill="#2dd4bf"
          />
        ))}

        {/* Hover vertical line */}
        {hover && (
          <line
            x1={hover.x}
            x2={hover.x}
            y1={PADDING_T}
            y2={H - PADDING_B}
            stroke="rgba(255,255,255,0.3)"
            strokeWidth={1}
            strokeDasharray="3 3"
          />
        )}

        {/* X labels (first, middle, last) */}
        <text x={xFor(0)} y={H - 8} textAnchor="start" fontSize={10} fill="#888">
          {new Date(points[0].t).toLocaleDateString()}
        </text>
        <text
          x={xFor(points.length - 1)}
          y={H - 8}
          textAnchor="end"
          fontSize={10}
          fill="#888"
        >
          {new Date(points[points.length - 1].t).toLocaleDateString()}
        </text>
      </svg>

      {hover && (
        <div className="ee-curve__tooltip">
          <div className="ee-curve__tooltip-time">{hover.p.label}</div>
          <div>
            <strong>Equity:</strong> {fmtMoney(hover.p.equity)}
          </div>
          <div>
            <strong>HWM:</strong> {fmtMoney(hover.p.hwm)}
          </div>
          {hover.p.floor !== null && (
            <div>
              <strong>Floor:</strong> {fmtMoney(hover.p.floor)}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
