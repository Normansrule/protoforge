// Catches rendering errors in a page so one bad input or lab bug never
// blanks the whole app; offers to reset the project if it is the cause.
import { Component, type ErrorInfo, type ReactNode } from 'react';
import { useProject } from '../state/project';

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('Protoforge page error:', error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="card" style={{ maxWidth: 720 }}>
        <h2>Something went wrong on this page</h2>
        <p className="muted">{this.state.error.message}</p>
        <p>Your project is still saved. You can try again, or reset to the Iris reference project if the project data itself is the problem.</p>
        <div className="row" style={{ marginTop: 12 }}>
          <button className="btn" onClick={() => this.setState({ error: null })}>Try again</button>
          <button
            className="btn primary"
            onClick={() => {
              useProject.getState().reset();
              this.setState({ error: null });
            }}
          >
            Reset to Iris
          </button>
        </div>
      </div>
    );
  }
}
