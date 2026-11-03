import "./Skeleton.css";

export function SkeletonCard({ lines = 3 }: { lines?: number }) {
  return (
    <div className="card">
      <div className="skeleton skeleton-line" style={{ width: "40%", height: 14 }} />
      <div style={{ height: 10 }} />
      {Array.from({ length: lines }).map((_, i) => (
        <div key={i} className="skeleton skeleton-line" style={{ width: `${90 - i * 15}%` }} />
      ))}
    </div>
  );
}

export function SkeletonChart({ height = 220 }: { height?: number }) {
  return (
    <div className="card">
      <div className="skeleton skeleton-line" style={{ width: "30%", height: 14 }} />
      <div style={{ height: 14 }} />
      <div className="skeleton" style={{ height, borderRadius: 8 }} />
    </div>
  );
}

export function SkeletonRows({ rows = 5 }: { rows?: number }) {
  return (
    <div className="card">
      <div className="skeleton skeleton-line" style={{ width: "35%", height: 14 }} />
      <div style={{ height: 14 }} />
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="skeleton skeleton-line" style={{ height: 24, marginBottom: 6 }} />
      ))}
    </div>
  );
}
