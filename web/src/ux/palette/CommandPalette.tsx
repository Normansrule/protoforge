// Command palette (Ctrl/⌘ K, "/" outside text fields, or the top-bar button
// via the `protoforge:palette` window event): jump to any lab, tab,
// requirement, metric, glossary term, guide or task, or run an action.
// The dialog is bundled with the app shell (not lazy) so keystrokes typed
// straight after Ctrl K are never lost while a chunk downloads; the search
// index itself is built only while the dialog is open.
import { useCallback, useEffect, useRef, useState } from 'react';
import { PALETTE_EVENT } from '../prefs';
import PaletteDialog from './PaletteDialog';

function isTyping(el: EventTarget | null): boolean {
  const n = el as HTMLElement | null;
  if (!n || !n.tagName) return false;
  const tag = n.tagName.toLowerCase();
  return tag === 'input' || tag === 'textarea' || tag === 'select' || n.isContentEditable;
}

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const restore = useRef<HTMLElement | null>(null);

  const show = useCallback((q = '') => {
    restore.current = document.activeElement as HTMLElement | null;
    setQuery(q);
    setOpen(true);
  }, []);
  const close = useCallback(() => {
    setOpen(false);
    const el = restore.current;
    // Give the route change (if any) a frame, then put focus back where it was.
    requestAnimationFrame(() => {
      if (el && document.contains(el)) el.focus();
    });
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        if (open) close();
        else show();
      } else if (e.key === '/' && !e.ctrlKey && !e.metaKey && !e.altKey && !open && !isTyping(e.target)) {
        e.preventDefault();
        show();
      }
    };
    const onEvent = (e: Event) => show((e as CustomEvent<{ query?: string }>).detail?.query ?? '');
    window.addEventListener('keydown', onKey);
    window.addEventListener(PALETTE_EVENT, onEvent);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener(PALETTE_EVENT, onEvent);
    };
  }, [open, show, close]);

  if (!open) return null;
  return <PaletteDialog initialQuery={query} onClose={close} />;
}
