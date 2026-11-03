import { useEffect, useRef } from "react";

/**
 * Adds `is-settled` after the first animation ends, so we can drop
 * `will-change`. Keeps GPU layers off after entry animations.
 */
export function useSettle<T extends HTMLElement>(delayMs = 400) {
  const ref = useRef<T | null>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const id = window.setTimeout(() => {
      el.classList.add("is-settled");
    }, delayMs);
    return () => window.clearTimeout(id);
  }, [delayMs]);
  return ref;
}
