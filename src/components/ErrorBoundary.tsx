import { Component, ErrorInfo, ReactNode } from "react";

interface Props { children: ReactNode }
interface State { error: Error | null; info: ErrorInfo | null }

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null, info: null };
  static getDerivedStateFromError(error: Error): Partial<State> { return { error }; }
  componentDidCatch(error: Error, info: ErrorInfo) { console.error("ErrorBoundary:", error, info); this.setState({ error, info }); }
  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: 24, fontFamily: "monospace", color: "#fca5a5", background: "#1a1a1a", minHeight: "100vh" }}>
          <h2 style={{ color: "#f87171" }}>Journal crashed</h2>
          <pre style={{ whiteSpace: "pre-wrap", fontSize: 12 }}>{this.state.error.message}{"\n\n"}{this.state.error.stack}</pre>
          {this.state.info && <pre style={{ whiteSpace: "pre-wrap", fontSize: 11, color: "#888" }}>{this.state.info.componentStack}</pre>}
        </div>
      );
    }
    return this.props.children;
  }
}
