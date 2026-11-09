import "./charts.css";

interface Bar { label: string; v: number; }
interface Props {
  data: Bar[];
  positiveColor?: string;
  negativeColor?: string;
  height?: number;
}

export function BarChart({
  data, positiveColor = "var(--accent)", negativeColor = "var(--danger)", height = 160,
}: Props) {
  if (!data.length) return <div className="chart-empty">No data</div>;

  const W = 600, H = height, P = 8;
  const maxAbs = Math.max(...data.map((d) => Math.abs(d.v)), 1);
  const zeroY = H / 2;
  const barW = (W - P * 2) / data.length;

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="chart" preserveAspectRatio="none">
      <line x1={P} x2={W - P} y1={zeroY} y2={zeroY} stroke="var(--border)" strokeWidth="1" />
      {data.map((d, i) => {
        const h = (Math.abs(d.v) / maxAbs) * (H / 2 - P);
        const x = P + i * barW + barW * 0.15;
        const w = barW * 0.7;
        const y = d.v >= 0 ? zeroY - h : zeroY;
        return (
          <g key={d.label}>
            <rect x={x} y={y} width={w} height={h}
              fill={d.v >= 0 ? positiveColor : negativeColor} rx="2" opacity="0.85" />
            <text x={x + w / 2} y={H - 1} textAnchor="middle" className="chart__axis">
              {d.label}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
