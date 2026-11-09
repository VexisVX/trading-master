// Professional market icons — sharp geometry, trader-grade.
// Not emoji, not cartoon. Angular bull/bear silhouettes.

interface IconProps { size?: number; className?: string; }

export function BullIcon({ size = 20, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      {/* Sharp upward horns */}
      <path d="M3 3 L6 8 L8 5 L10 9" />
      <path d="M21 3 L18 8 L16 5 L14 9" />
      {/* Head silhouette — angular */}
      <path d="M8 9 L12 8 L16 9 L17 14 L15 18 L12 20 L9 18 L7 14 Z" />
      {/* Eyes — slanted */}
      <path d="M9.5 13 L11 14" />
      <path d="M14.5 13 L13 14" />
      {/* Snout */}
      <path d="M11 17 L13 17" />
    </svg>
  );
}

export function BearIcon({ size = 20, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      {/* Sharp downward ears */}
      <path d="M4 4 L6 8 L7 6" />
      <path d="M20 4 L18 8 L17 6" />
      {/* Head silhouette — wider, lower */}
      <path d="M6 9 L9 8 L15 8 L18 9 L19 14 L17 18 L12 20 L7 18 L5 14 Z" />
      {/* Eyes — slanted down */}
      <path d="M9 12 L10.5 13" />
      <path d="M15 12 L13.5 13" />
      {/* Snout */}
      <circle cx="12" cy="17" r="1" fill="currentColor" />
    </svg>
  );
}

export function NeutralIcon({ size = 20, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" className={className} fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      {/* Simple dot — low importance */}
      <circle cx="12" cy="12" r="4" />
      <circle cx="12" cy="12" r="1" fill="currentColor" />
    </svg>
  );
}
