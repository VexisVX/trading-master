import "./charts.css";

interface Point { t: number; v: number; }
interface Props {
  data: Point[];
  color?: string;
  fill?: boolean;
  height?: number;
  formatY?: (v: number) => string;
}

export function LineChart({
  data, color = "var(--accent)", fill = true, height = 220, formatY,
}: Props) {
  if (data.length < 2) return <div className="chart-empty">No data</div>;

  const W = 800, H = height, P = 12;
  const xs = data.map((d) => d.t);
  const ys = data.map((d) => d.v);
  const minX = Math.min(...xs), maxX = Math.max(...xs);
  const minY = Math.min(...ys), maxY = Math.max(...ys);
  const spanY = maxY - minY || 1;

  const sx = (x: number) => P + ((x - minX) / (maxX - minX || 1)) * (W - P * 2);
  const sy = (y: number) => H - P - ((y - minY) / spanY) * (H - P * 2);

  const line = data.map((d, i) => `${i === 0 ? "M" : "L"}${sx(d.t)},${sy(d.v)}`).join(" ");
  const area = `${line} L${sx(maxX)},${H - P} L${sx(minX)},${H - P} Z`;
  const gradId = `grad-${color.replace(/[^a-z]/gi, "")}`;

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="chart" preserveAspectRatio="none">
      <defs>
        <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity="0.28" />
          <stop offset="100%" stopColor={color} stopOpacity="0" />
        </linearGradient>
      </defs>
      {fill && <path d={area} fill={`url(#${gradId})`} />}
      <path d={line} fill="none" stroke={color} strokeWidth="2" />
      {formatY && (
        <>
          <text x={P} y={P + 8} className="chart__axis">{formatY(maxY)}</text>
          <text x={P} y={H - P - 2} className="chart__axis">{formatY(minY)}</text>
        </>
      )}
    </svg>
  );
}
