import { useMemo } from "react";
import { EquityPoint, parseR } from "../../lib/backtestApi";

interface Props {
  points: EquityPoint[];
}

const W = 640;
const H = 200;
const PAD_L = 36;
const PAD_R = 12;
const PAD_T = 12;
const PAD_B = 24;

export function EquityChart({ points }: Props) {
  const path = useMemo(() => {
    if (points.length < 2) return null;
    const rs = points.map((p) => parseR(p.r) ?? 0);
    const min = Math.min(0, ...rs);
    const max = Math.max(0, ...rs);
    const span = max - min || 1;
    const x = (i: number) =>
      PAD_L + (i / (points.length - 1)) * (W - PAD_L - PAD_R);
    const y = (v: number) =>
      PAD_T + (1 - (v - min) / span) * (H - PAD_T - PAD_B);
    let d = "";
    points.forEach((p, i) => {
      const v = parseR(p.r) ?? 0;
      d += (i === 0 ? "M" : "L") + x(i).toFixed(1) + "," + y(v).toFixed(1);
    });
    const zeroY = y(0);
    return { d, min, max, zeroY, x, y };
  }, [points]);

  if (!path) {
    return <div className="bt-empty bt-empty--chart">Not enough trades yet.</div>;
  }

  const last = parseR(points[points.length - 1].r) ?? 0;
  const color = last >= 0 ? "#4ade80" : "#f87171";
  const fillColor = last >= 0 ? "rgba(74,222,128,0.12)" : "rgba(248,113,113,0.12)";

  // Build filled area
  const firstX = path.x(0).toFixed(1);
  const lastX = path.x(points.length - 1).toFixed(1);
  const zeroY = path.zeroY.toFixed(1);
  const area = `${path.d}L${lastX},${zeroY}L${firstX},${zeroY}Z`;

  // Y-axis ticks
  const ticks = 4;
  const tickVals = Array.from({ length: ticks + 1 }, (_, i) =>
    path.min + ((path.max - path.min) * i) / ticks
  );

  return (
    <svg className="bt-svg" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none">
      {/* zero line */}
      <line
        x1={PAD_L}
        x2={W - PAD_R}
        y1={path.zeroY}
        y2={path.zeroY}
        stroke="rgba(255,255,255,0.12)"
        strokeDasharray="2 3"
      />
      {/* y ticks */}
      {tickVals.map((v, i) => {
        const y = path.y(v);
        return (
          <g key={i}>
            <text
              x={PAD_L - 6}
              y={y + 3}
              textAnchor="end"
              fontSize="9"
              fill="#888"
            >
              {v.toFixed(1)}
            </text>
          </g>
        );
      })}
      {/* area */}
      <path d={area} fill={fillColor} />
      {/* line */}
      <path d={path.d} fill="none" stroke={color} strokeWidth="1.6" />
    </svg>
  );
}
