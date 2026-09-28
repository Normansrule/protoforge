import { useEffect, useState } from 'react';

/** true when the user asked the operating system for reduced motion. */
export function reducedMotion(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

/** Live `prefers-reduced-motion` flag. */
export function useReducedMotion(): boolean {
  const [r, setR] = useState(reducedMotion);
  useEffect(() => {
    const mq = window.matchMedia?.('(prefers-reduced-motion: reduce)');
    if (!mq) return;
    const on = () => setR(mq.matches);
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  }, []);
  return r;
}

/** Becomes true once the element has scrolled near the viewport (or immediately without IntersectionObserver). */
export function useNearViewport(ref: React.RefObject<Element | null>, margin = '200px'): boolean {
  const [seen, setSeen] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!el || seen) return;
    if (typeof IntersectionObserver === 'undefined') {
      setSeen(true);
      return;
    }
    const io = new IntersectionObserver((es) => es.some((e) => e.isIntersecting) && setSeen(true), { rootMargin: margin });
    io.observe(el);
    // Never leave content hidden (e.g. full-page captures, printing).
    const t = setTimeout(() => setSeen(true), 2500);
    return () => {
      io.disconnect();
      clearTimeout(t);
    };
  }, [ref, seen, margin]);
  return seen;
}

/** Count up from 0 to `target` once (eased), respecting reduced motion. */
export function useCountUp(target: number, ms = 900): number {
  const [v, setV] = useState(reducedMotion() ? target : 0);
  useEffect(() => {
    if (reducedMotion()) {
      setV(target);
      return;
    }
    let raf = 0;
    const t0 = performance.now();
    const from = 0;
    const tick = (now: number) => {
      const k = Math.min(1, (now - t0) / ms);
      const e = 1 - Math.pow(1 - k, 3);
      setV(from + (target - from) * e);
      if (k < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [target, ms]);
  return v;
}
