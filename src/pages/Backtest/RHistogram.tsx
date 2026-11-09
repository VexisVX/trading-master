import { useMemo } from "react";
import { HistBucket } from "../../lib/backtestApi";

interface Props {
  buckets: HistBucket[];
}

const W = 640;
const H = 200;
const PAD_L = 28;
const PAD_R = 8;
const PAD_T = 12;
const PAD_B = 26;

export function RHistogram({ buckets }: Props) {
  const data = useMemo(() => {
    const max = Math.max(1, ...buckets.map((b) => b.count));
    const n = buckets.length || 1;
    const barW = (W - PAD_L - PAD_R) / n;
    return { max, barW, n };
  }, [buckets]);

  if (buckets.every((b) => b.count === 0)) {
    return <div className="bt-empty bt-empty--chart">No closed trades yet.</div>;
  }

  return (
    <svg className="bt-svg" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none">
      <line
        x1={PAD_L}
        x2={W - PAD_R}
        y1={H - PAD_B}
        y2={H - PAD_B}
        stroke="rgba(255,255,255,0.12)"
      />
      {buckets.map((b, i) => {
        const h = (b.count / data.max) * (H - PAD_T - PAD_B);
        const x = PAD_L + i * data.barW + data.barW * 0.12;
        const y = H - PAD_B - h;
        const w = data.barW * 0.76;
        // color: negative buckets red-ish, positive green-ish, zero gray
        const isNeg = b.label.startsWith("-") || b.label.startsWith("≤");
        const isPos = b.label.startsWith("+") || b.label.startsWith("≥");
        const fill = isNeg
          ? "rgba(248,113,113,0.55)"
          : isPos
          ? "rgba(74,222,128,0.55)"
          : "rgba(255,255,255,0.18)";
        return (
          <g key={b.label}>
            <rect x={x} y={y} width={w} height={h} fill={fill} rx={2} />
            {b.count > 0 && (
              <text
                x={x + w / 2}
                y={y - 4}
                textAnchor="middle"
                fontSize="9"
                fill="#aaa"
              >
                {b.count}
              </text>
            )}
            <text
              x={x + w / 2}
              y={H - PAD_B + 12}
              textAnchor="middle"
              fontSize="9"
              fill="#888"
            >
              {b.label}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
