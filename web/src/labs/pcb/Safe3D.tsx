// Keeps a lab page alive if the 3-D view fails (e.g. offline: the shared
// Scene3D fetches its environment map from a CDN).
import { Component, type ReactNode } from 'react';

export class Safe3D extends Component<{ children: ReactNode; height?: number }, { failed: string | null }> {
  state = { failed: null as string | null };
  static getDerivedStateFromError(e: unknown) {
    return { failed: e instanceof Error ? e.message : String(e) };
  }
  render() {
    if (this.state.failed) {
      return (
        <div className="scene3d" style={{ height: this.props.height ?? 300, display: 'grid', placeItems: 'center', padding: 20 }}>
          <div className="small muted" style={{ textAlign: 'center' }}>3-D view unavailable ({this.state.failed}).<br />The analysis results below are unaffected.</div>
        </div>
      );
    }
    return this.props.children;
  }
}
