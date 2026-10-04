// <Term k="MTF"/> — an acronym or term of art with its full form and a plain
// definition from the glossary, shown on hover, keyboard focus and tap.
import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Link } from 'react-router-dom';
import { lookupTerm } from '../data/terms';
import './help.css';

// Only one popover at a time: opening one closes the previous.
let closeOpen: (() => void) | null = null;

export function Term({ k, children, showFull }: { k: string; children?: ReactNode; /** Render "full form (K)" instead of just K. */ showFull?: boolean }) {
  const t = lookupTerm(k);
  const id = useId();
  const btn = useRef<HTMLButtonElement>(null);
  const tip = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [pinned, setPinned] = useState(false);
  const [pos, setPos] = useState<{ left: number; top: number; above: boolean } | null>(null);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const closeNow = useCallback(() => {
    setOpen(false);
    setPinned(false);
  }, []);
  const show = useCallback(() => {
    if (hideTimer.current) clearTimeout(hideTimer.current);
    if (closeOpen && closeOpen !== closeNow) closeOpen();
    closeOpen = closeNow;
    setOpen(true);
  }, [closeNow]);
  const hide = useCallback(() => {
    if (hideTimer.current) clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      setOpen(false);
      setPinned(false);
    }, 120);
  }, []);

  useLayoutEffect(() => {
    if (!open || !btn.current) return;
    const place = () => {
      const r = btn.current!.getBoundingClientRect();
      const w = Math.min(320, window.innerWidth - 16);
      const h = tip.current?.offsetHeight ?? 120;
      const left = Math.max(8, Math.min(r.left + r.width / 2 - w / 2, window.innerWidth - w - 8));
      const above = r.bottom + h + 12 > window.innerHeight && r.top - h - 12 > 0;
      setPos({ left, top: above ? r.top - h - 8 : r.bottom + 8, above });
    };
    place();
    window.addEventListener('scroll', place, true);
    window.addEventListener('resize', place);
    return () => {
      window.removeEventListener('scroll', place, true);
      window.removeEventListener('resize', place);
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpen(false);
        setPinned(false);
      }
    };
    const onDown = (e: PointerEvent) => {
      if (!pinned) return;
      const target = e.target as Node;
      if (btn.current?.contains(target) || tip.current?.contains(target)) return;
      setOpen(false);
      setPinned(false);
    };
    document.addEventListener('keydown', onKey);
    document.addEventListener('pointerdown', onDown);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('pointerdown', onDown);
    };
  }, [open, pinned]);

  useEffect(
    () => () => {
      if (hideTimer.current) clearTimeout(hideTimer.current);
      if (closeOpen === closeNow) closeOpen = null;
    },
    [closeNow],
  );

  const label = children ?? (showFull && t && t.full.toLowerCase() !== k.toLowerCase() ? `${t.full} (${k})` : k);
  if (!t) return <>{label}</>;
  return (
    <>
      <button
        ref={btn}
        type="button"
        className="ux-term"
        aria-describedby={open ? id : undefined}
        aria-expanded={open}
        onMouseEnter={show}
        onMouseLeave={() => !pinned && hide()}
        onFocus={show}
        onBlur={() => !pinned && hide()}
        onClick={() => {
          if (pinned) {
            setPinned(false);
            setOpen(false);
          } else {
            setPinned(true);
            show();
          }
        }}
      >
        {label}
      </button>
      {open &&
        createPortal(
          <div
            ref={tip}
            id={id}
            role="tooltip"
            className={`ux-term-tip ${pos?.above ? 'above' : ''}`}
            style={{ left: pos?.left ?? -9999, top: pos?.top ?? -9999, width: Math.min(320, window.innerWidth - 16) }}
            onMouseEnter={show}
            onMouseLeave={() => !pinned && hide()}
          >
            <div className="ux-term-head">
              <b>{t.term}</b>
              {t.full.toLowerCase() !== t.term.toLowerCase() && <span>{t.full}</span>}
            </div>
            <p>{t.def}</p>
            {t.lab && (
              <Link to={t.lab} className="ux-term-go" onClick={() => setOpen(false)} tabIndex={-1}>
                Where it is used →
              </Link>
            )}
          </div>,
          document.body,
        )}
    </>
  );
}
