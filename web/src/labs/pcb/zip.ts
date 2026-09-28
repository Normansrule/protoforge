// Bundle text/base64 files into a zip for download.
import JSZip from 'jszip';
import { downloadBlob } from '../../components/ui';

export interface ZipFile {
  path: string;
  content: string;
  encoding?: string;
}

export async function downloadZip(filename: string, files: ZipFile[]) {
  const zip = new JSZip();
  for (const f of files) {
    if (f.encoding === 'base64') zip.file(f.path, f.content, { base64: true });
    else zip.file(f.path, f.content);
  }
  const blob = await zip.generateAsync({ type: 'blob', compression: 'DEFLATE' });
  downloadBlob(filename, blob);
}

/** Respect the user's reduced-motion preference for all animations. */
export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}
