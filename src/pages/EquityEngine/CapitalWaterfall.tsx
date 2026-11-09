import { fmtMoney, parseDec } from "../../lib/equityApi";

interface Props {
  equity: string;
  initialBalance: string;
  floor: string | null;
  protectedProfit: string | null;
  lossCapacity: string | null;
}

export function CapitalWaterfall({
  equity,
  initialBalance,
  floor,
  protectedProfit,
  lossCapacity,
}: Props) {
  const eq = parseDec(equity) ?? 0;
  const init = parseDec(initialBalance) ?? 0;
  const fl = parseDec(floor ?? undefined);
  const prot = parseDec(protectedProfit ?? undefined);
  const lc = parseDec(lossCapacity ?? undefined);

  // Guard: bail out on invalid data
  if (!isFinite(eq) || !isFinite(init) || eq <= 0 || init <= 0) {
    return (
      <div className="ee-empty-inline">
        Snapshot data unavailable — click ⟳ Refresh.
      </div>
    );
  }

  // Range for bar chart: from min(initial, floor) to max(equity, hwm)
  const minV = Math.min(init, fl ?? init, eq);
  const maxV = Math.max(eq, init);
  const range = maxV - minV || 1;
  const pad = range * 0.1;
  const yMin = minV - pad;
  const yMax = maxV + pad;

  const W = 340;
  const H = 260;
  const PAD_L = 12;
  const PAD_R = 12;
  const PAD_T = 16;
  const PAD_B = 40;
  const chartW = W - PAD_L - PAD_R;
  const chartH = H - PAD_T - PAD_B;

  const yFor = (v: number) =>
    PAD_T + chartH - ((v - yMin) / (yMax - yMin)) * chartH;

  // Bars to render, from bottom (low value) to top (high value)
  const bars: {
    label: string;
    value: number;
    color: string;
    from: number;
    to: number;
  }[] = [];

  // Protected profit band (from initial to floor)
  if (fl !== null && fl > init) {
    bars.push({
      label: "🛡 Protected",
      value: fl - init,
      color: "#4ade80",
      from: init,
      to: fl,
    });
  } else if (prot !== null && prot > 0) {
    bars.push({
      label: "🛡 Protected",
      value: prot,
      color: "#4ade80",
      from: init,
      to: init + prot,
    });
  }

  // Loss capacity band (from floor to equity)
  if (fl !== null && eq > fl) {
    bars.push({
      label: "📊 Loss capacity",
      value: eq - fl,
      color: "#2dd4bf",
      from: fl,
      to: eq,
    });
  } else if (lc !== null && lc > 0) {
    bars.push({
      label: "📊 Loss capacity",
      value: lc,
      color: "#2dd4bf",
      from: eq,
      to: eq + lc,
    });
  }

  const barW = chartW * 0.7;
  const barX = PAD_L + (chartW - barW) / 2;

  return (
    <div className="ee-waterfall">
      <svg viewBox={`0 0 ${W} ${H}`} className="ee-waterfall__svg">
        {/* Grid */}
        {[init, fl, eq].map((v, i) =>
          v !== null && isFinite(v) ? (
            <line
              key={i}
              x1={PAD_L}
              x2={W - PAD_R}
              y1={yFor(v)}
              y2={yFor(v)}
              stroke="rgba(255,255,255,0.08)"
              strokeDasharray="2 3"
            />
          ) : null
        )}

        {/* Bars (stacked visually) */}
        {bars.map((b, i) => {
          const yTop = yFor(b.to);
          const yBot = yFor(b.from);
          const height = Math.abs(yBot - yTop);
          return (
            <g key={i}>
              <rect
                x={barX}
                y={Math.min(yTop, yBot)}
                width={barW}
                height={height}
                fill={b.color}
                opacity={0.85}
                rx={2}
              />
              <text
                x={barX + barW / 2}
                y={Math.min(yTop, yBot) + height / 2}
                textAnchor="middle"
                dominantBaseline="middle"
                fontSize={11}
                fontWeight={600}
                fill="#0b0f14"
              >
                {fmtMoney(b.value)}
              </text>
            </g>
          );
        })}

        {/* Reference lines with labels */}
        <LineLabel
          x1={PAD_L}
          x2={W - PAD_R}
          y={yFor(eq)}
          label={`Equity ${fmtMoney(eq)}`}
          color="#2dd4bf"
          above
        />
        {fl !== null && (
          <LineLabel
            x1={PAD_L}
            x2={W - PAD_R}
            y={yFor(fl)}
            label={`Floor ${fmtMoney(fl)}`}
            color="#ff6b6b"
            above={false}
          />
        )}
        <LineLabel
          x1={PAD_L}
          x2={W - PAD_R}
          y={yFor(init)}
          label={`Initial ${fmtMoney(init)}`}
          color="#888"
          above={false}
        />
      </svg>

      <div className="ee-waterfall__legend">
        {bars.map((b, i) => (
          <div key={i} className="ee-waterfall__legend-item">
            <span
              className="ee-waterfall__legend-swatch"
              style={{ background: b.color }}
            />
            <span>{b.label}:</span>
            <strong>{fmtMoney(b.value)}</strong>
          </div>
        ))}
      </div>
    </div>
  );
}

function LineLabel({
  x1,
  x2,
  y,
  label,
  color,
  above,
}: {
  x1: number;
  x2: number;
  y: number;
  label: string;
  color: string;
  above: boolean;
}) {
  return (
    <g>
      <line x1={x1} x2={x2} y1={y} y2={y} stroke={color} strokeWidth={1.5} />
      <text
        x={x2 - 4}
        y={above ? y - 4 : y + 12}
        textAnchor="end"
        fontSize={10}
        fill={color}
        fontWeight={600}
      >
        {label}
      </text>
    </g>
  );
}
