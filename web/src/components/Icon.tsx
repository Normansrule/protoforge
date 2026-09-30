// Minimal stroke icon set (24×24 grid) so the app has no icon-font dependency.
const P: Record<string, string> = {
  home: 'M3 11l9-8 9 8M5 10v10h14V10',
  systems: 'M4 4h6v6H4zM14 14h6v6h-6zM14 4h6v6h-6zM7 10v4h7M17 10v4',
  circuit: 'M2 12h4l2-5 3 10 3-10 2 5h6',
  power: 'M13 2L4 14h7l-1 8 9-12h-7z',
  pcb: 'M4 4h16v16H4zM8 8h3v3H8zM13 13h3v3h-3zM11 9.5h5V13M8 11v5h5',
  dsp: 'M2 12c2-6 4-6 6 0s4 6 6 0 4-6 6 0 2 3 2 3',
  chip: 'M7 7h10v10H7zM10 3v4M14 3v4M10 17v4M14 17v4M3 10h4M3 14h4M17 10h4M17 14h4',
  control: 'M3 17l4-4 3 3 5-6 6 5M3 21h18',
  robotics: 'M12 3v4M8 7h8v6H8zM10 10h.01M14 10h.01M6 13l-3 5M18 13l3 5M9 13v5h6v-5',
  rl: 'M4 20V10M10 20V4M16 20v-7M22 20H2',
  optics: 'M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12zM12 9a3 3 0 110 6 3 3 0 010-6z',
  cad: 'M12 2l9 5v10l-9 5-9-5V7zM12 22V12M21 7l-9 5-9-5',
  print: 'M6 3h12v5H6zM4 8h16v8H4zM8 16v5h8v-5M8 12h8',
  molding: 'M3 6h18v5H3zM3 13h18v5H3zM9 11v2M15 11v2',
  mfg: 'M3 21V10l6 4V10l6 4V6l6 4v11zM7 17h2M12 17h2M17 17h2',
  learn: 'M4 5h7a3 3 0 013 3v13a2 2 0 00-2-2H4zM20 5h-7a3 3 0 00-3 3',
  structures: 'M3 20h18M5 20l7-14 7 14M8.5 13h7M12 6v14',
  quantum: 'M12 12m-1.5 0a1.5 1.5 0 103 0 1.5 1.5 0 10-3 0M3 12c0-3 4-5 9-5s9 2 9 5-4 5-9 5-9-2-9-5zM7.5 4.2c2.6-1.5 7 1.3 9.5 5.6s2.6 9.2 0 10.7-7-1.3-9.5-5.6-2.6-9.2 0-10.7z',
  proto: 'M4 7h16v10H4zM7 7V4M11 7V4M15 7V4M7 17v3M11 17v3M15 17v3M8 10h3v4H8zM14 11h2',
  build: 'M14.7 6.3a4 4 0 00-5.4 5.4L3 18l3 3 6.3-6.3a4 4 0 005.4-5.4l-2.5 2.5-2.4-.6-.6-2.4z',
  toolchain: 'M4 6h6v5H4zM14 13h6v5h-6zM10 8.5h4v7M4 16h6M14 8h6',
  versions: 'M6 3v12M6 15a3 3 0 103 3M18 9a3 3 0 10-3-3M18 9c0 4-6 3-9 7M6 6h.01',
  docs: 'M6 3h9l4 4v14H6zM14 3v5h5M9 12h7M9 16h7',
  rf: 'M12 20V11M8 6a6 6 0 018 0M5 3a10 10 0 0114 0M12 11l-4 9M12 11l4 9M12 9a1 1 0 110 2 1 1 0 010-2z',
  fluids: 'M12 3c3 4 5 6.5 5 9.5a5 5 0 01-10 0C7 9.5 9 7 12 3zM3 20c2-1 4-1 6 0s4 1 6 0 4-1 6 0',
  mechanisms: 'M12 9a3 3 0 110 6 3 3 0 010-6zM12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9L7 7M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1',
  ergonomics: 'M12 3a4 4 0 110 8 4 4 0 010-8zM4 21c0-4 3.6-7 8-7s8 3 8 7',
  reliability: 'M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6zM8.5 12l2.5 2.5 4.5-5',
  program: 'M4 5h8M4 10h12M9 15h11M6 20h7M4 3v18',
  sun: 'M12 4V2M12 22v-2M4 12H2M22 12h-2M5 5l1.5 1.5M17.5 17.5L19 19M5 19l1.5-1.5M17.5 6.5L19 5M12 7a5 5 0 110 10 5 5 0 010-10z',
  moon: 'M20 14.5A8 8 0 019.5 4 8 8 0 1020 14.5z',
  menu: 'M3 6h18M3 12h18M3 18h18',
  github: 'M9 19c-4 1.5-4-2-6-2.5m12 5v-3.5a3 3 0 00-.8-2.3c2.7-.3 5.6-1.3 5.6-6a4.7 4.7 0 00-1.3-3.2 4.3 4.3 0 00-.1-3.2s-1-.3-3.3 1.3a11.4 11.4 0 00-6 0C6.8 2.9 5.8 3.2 5.8 3.2a4.3 4.3 0 00-.1 3.2A4.7 4.7 0 004.4 9.6c0 4.6 2.8 5.7 5.5 6a3 3 0 00-.8 2.3V22',
  play: 'M7 4l13 8-13 8z',
  download: 'M12 3v12M7 10l5 5 5-5M4 21h16',
  upload: 'M12 21V9M7 14l5-5 5 5M4 3h16',
  reset: 'M3 12a9 9 0 109-9 9.7 9.7 0 00-6.7 2.8L3 8M3 3v5h5',
  check: 'M4 12l5 5L20 6',
  arrow: 'M5 12h14M13 6l6 6-6 6',
};

export function Icon({ name, size = 18, className }: { name: string; size?: number; className?: string }) {
  return (
    <svg className={className} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <path d={P[name] ?? P.systems} />
    </svg>
  );
}
