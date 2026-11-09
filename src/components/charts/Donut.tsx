import "./charts.css";

interface Props {
  wins: number;
  losses: number;
  size?: number;
}

export function Donut({ wins, losses, size = 140 }: Props) {
  const total = wins + losses || 1;
  const winPct = wins / total;
  const R = size / 2 - 10;
  const C = 2 * Math.PI * R;
  const stroke = 14;
  const dash = C * winPct;

  return (
    <div className="donut-wrap">
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <circle cx={size / 2} cy={size / 2} r={R}
          fill="none" stroke="var(--danger)" strokeWidth={stroke} opacity="0.25" />
        <circle cx={size / 2} cy={size / 2} r={R}
          fill="none" stroke="var(--accent)" strokeWidth={stroke}
          strokeDasharray={`${dash} ${C - dash}`}
          strokeLinecap="round"
          transform={`rotate(-90 ${size / 2} ${size / 2})`} />
        <text x="50%" y="47%" textAnchor="middle" className="donut__value">
          {Math.round(winPct * 100)}%
        </text>
        <text x="50%" y="60%" textAnchor="middle" className="donut__label">
          win rate
        </text>
      </svg>
    </div>
  );
}
