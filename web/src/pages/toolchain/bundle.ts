// The toolchain bundle (bridge.export_all), built once per project version in
// the engine worker, and helpers to download slices of it.
import { useCallback, useState } from 'react';
import type { Project } from '../../engine/types';
import { workerCall } from '../../labs/pcb/useWorker';
import { downloadZip } from '../../labs/pcb/zip';
import { downloadBlob } from '../../components/ui';
import type { DownloadSpec } from './tools';
import type { BundleFile } from './types';

const cache = new WeakMap<Project, Promise<BundleFile[]>>();

export function useBundle(project: Project) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const get = useCallback(() => {
    let p = cache.get(project);
    if (!p) {
      const timestamp = new Date().toISOString().slice(0, 19);
      p = workerCall<BundleFile[]>('bridge.export_all', { project, options: { gcode: true, timestamp } });
      cache.set(project, p);
      p.catch(() => cache.delete(project));
    }
    setBusy(true);
    setError(null);
    return p
      .catch((e: Error): BundleFile[] => {
        setError(e.message);
        return [];
      })
      .finally(() => setBusy(false));
  }, [project]);
  return { get, busy, error };
}

function bytes(f: BundleFile): Blob {
  if (f.encoding === 'base64') {
    const bin = atob(f.content);
    const a = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) a[i] = bin.charCodeAt(i);
    return new Blob([a], { type: 'application/octet-stream' });
  }
  return new Blob([f.content], { type: 'text/plain' });
}

/** Download the files of the bundle that match a spec (zip or single file). */
export async function downloadFiles(files: BundleFile[], d: DownloadSpec) {
  const sel = files.filter((f) => d.match(f.path));
  if (sel.length === 0) return;
  if (!d.zip && sel.length === 1) {
    downloadBlob(d.filename, bytes(sel[0]));
    return;
  }
  await downloadZip(d.filename, sel);
}
