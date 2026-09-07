import { useEffect, useRef, useState } from "react";

function prefersReducedMotion(): boolean {
  try {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  } catch {
    return true;
  }
}

/**
 * Animates a known, already-approved numeric value from 0 up to itself.
 * Never invents or interpolates unapproved figures — every intermediate
 * frame is the same formatter applied to a smaller real number, and the
 * animation always lands on the exact final formatted string.
 *
 * Falls back to rendering the final value immediately when reduced motion
 * is requested or `requestAnimationFrame` is unavailable (e.g. jsdom), so
 * tests and accessibility-sensitive users see the settled value with no
 * timing dependency.
 */
export function CountUp({
  value,
  format,
  durationMs = 600,
}: {
  value: number;
  format: (value: number) => string;
  durationMs?: number;
}) {
  const [display, setDisplay] = useState(value);
  const previousValue = useRef(value);

  useEffect(() => {
    const from = previousValue.current;
    const to = value;
    previousValue.current = value;

    if (from === to || prefersReducedMotion() || typeof requestAnimationFrame !== "function") {
      setDisplay(to);
      return;
    }

    let frame = 0;
    const start = performance.now();
    const tick = (now: number) => {
      const elapsed = now - start;
      const progress = Math.min(1, elapsed / durationMs);
      const eased = 1 - (1 - progress) ** 3;
      setDisplay(Math.round(from + (to - from) * eased));
      if (progress < 1) {
        frame = requestAnimationFrame(tick);
      } else {
        setDisplay(to);
      }
    };
    frame = requestAnimationFrame(tick);

    return () => {
      if (typeof cancelAnimationFrame === "function") cancelAnimationFrame(frame);
    };
  }, [value, durationMs]);

  return <>{format(display)}</>;
}
