// Track when we hit 429 per API to prevent spamming
const last429: Record<string, number> = {};

export function mark429(api: string) {
  last429[api] = Date.now();
}

export function isRateLimited(api: string): { limited: boolean; retryInSec: number } {
  const ts = last429[api];
  if (!ts) return { limited: false, retryInSec: 0 };
  const elapsed = (Date.now() - ts) / 1000;
  // Assume reset in 60s for per-minute, or much longer for daily
  // Conservative: 60 seconds
  if (elapsed < 60) {
    return { limited: true, retryInSec: Math.ceil(60 - elapsed) };
  }
  return { limited: false, retryInSec: 0 };
}
