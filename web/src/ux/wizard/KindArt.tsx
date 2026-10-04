// Small line illustrations for the product kinds (96 × 64 grid). Stroke is
// the current text colour; the accent marks what makes the kind distinctive.
import type { ReactNode } from 'react';

const A = 'var(--accent)';
const A2 = 'var(--accent-2)';

const ART: Record<string, ReactNode> = {
  wearable: (
    <>
      <path d="M38 6h20l3 14H35zM35 44h26l-3 14H38z" />
      <rect x="30" y="18" width="36" height="28" rx="9" />
      <rect x="36" y="24" width="24" height="16" rx="4" fill={A} fillOpacity="0.18" stroke={A} />
      <path d="M41 33l4-4 3 3 5-5" stroke={A} />
      <path d="M68 28v8" />
    </>
  ),
  handheld: (
    <>
      <rect x="30" y="4" width="36" height="56" rx="7" />
      <rect x="35" y="10" width="26" height="22" rx="2" fill={A} fillOpacity="0.18" stroke={A} />
      <circle cx="42" cy="44" r="4" />
      <circle cx="55" cy="40" r="2.5" stroke={A2} />
      <circle cx="58" cy="48" r="2.5" stroke={A2} />
      <path d="M45 55h6" />
    </>
  ),
  iot_sensor: (
    <>
      <circle cx="40" cy="38" r="17" />
      <circle cx="40" cy="38" r="7" fill={A} fillOpacity="0.18" stroke={A} />
      <path d="M60 22a14 14 0 0 1 0 14M66 16a22 22 0 0 1 0 26M72 10a30 30 0 0 1 0 38" stroke={A} />
      <path d="M33 58h14" />
    </>
  ),
  desk_instrument: (
    <>
      <rect x="18" y="8" width="60" height="38" rx="5" />
      <rect x="24" y="14" width="34" height="26" rx="2" fill={A} fillOpacity="0.18" stroke={A} />
      <path d="M28 33l6-8 5 5 6-10 6 13" stroke={A} />
      <circle cx="68" cy="20" r="3" />
      <circle cx="68" cy="32" r="3" />
      <path d="M26 46l-4 10M70 46l4 10M20 58h56" />
    </>
  ),
  robot: (
    <>
      <path d="M48 4v8" />
      <circle cx="48" cy="4" r="2" stroke={A} />
      <rect x="28" y="12" width="40" height="30" rx="8" />
      <circle cx="40" cy="26" r="3.5" fill={A} stroke={A} />
      <circle cx="56" cy="26" r="3.5" fill={A} stroke={A} />
      <path d="M42 35h12" />
      <circle cx="30" cy="50" r="9" />
      <circle cx="66" cy="50" r="9" />
      <circle cx="30" cy="50" r="2" stroke={A2} />
      <circle cx="66" cy="50" r="2" stroke={A2} />
    </>
  ),
  smart_home: (
    <>
      <path d="M16 32L48 8l32 24M24 27v29h48V27" />
      <rect x="40" y="38" width="16" height="18" rx="2" />
      <path d="M40 22a11 11 0 0 1 16 0M43 26a6 6 0 0 1 10 0" stroke={A} />
      <circle cx="48" cy="30" r="1.5" fill={A} stroke={A} />
    </>
  ),
  optical: (
    <>
      <rect x="6" y="24" width="16" height="16" rx="3" />
      <path d="M22 28l26-8M22 36l26 8" stroke={A} strokeDasharray="3 3" />
      <path d="M48 12c6 6 6 34 0 40c-6-6-6-34 0-40z" fill={A} fillOpacity="0.18" stroke={A} />
      <path d="M50 20l36 12-36 12" stroke={A} />
      <rect x="84" y="26" width="6" height="12" rx="1" />
    </>
  ),
  quantum: (
    <>
      <rect x="4" y="20" width="22" height="24" rx="4" />
      <rect x="70" y="20" width="22" height="24" rx="4" />
      <path d="M26 32c6 0 6-14 12-14s6 28 12 28 6-28 12-28 6 14 8 14" stroke={A} />
      <circle cx="44" cy="32" r="2.5" fill={A2} stroke={A2} />
      <path d="M10 28h10M10 34h6M76 28h10M76 34h6" />
    </>
  ),
  dev_board: (
    <>
      <rect x="12" y="14" width="72" height="36" rx="3" />
      <rect x="40" y="24" width="16" height="16" rx="1" fill={A} fillOpacity="0.18" stroke={A} />
      <path d="M18 14v-5M24 14v-5M30 14v-5M66 14v-5M72 14v-5M78 14v-5M18 50v5M24 50v5M30 50v5M66 50v5M72 50v5M78 50v5" />
      <rect x="14" y="27" width="10" height="10" rx="1" stroke={A2} />
      <path d="M56 32h10M30 32h10" stroke={A} />
    </>
  ),
  mechanical: (
    <>
      <path d="M20 8v48h8V16h48v-8z" fill={A} fillOpacity="0.12" />
      <path d="M20 8h56v8H28v40h-8z" />
      <circle cx="24" cy="30" r="2" />
      <circle cx="24" cy="44" r="2" />
      <path d="M70 22v20M64 36l6 6 6-6" stroke={A} />
      <path d="M12 4v58" strokeDasharray="2 3" />
    </>
  ),
};

export function KindArt({ id, size = 88 }: { id: string; size?: number }) {
  return (
    <svg viewBox="0 0 96 64" width={size} height={(size * 64) / 96} fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {ART[id] ?? ART.handheld}
    </svg>
  );
}
