import "./Sidebar.css";

export type Page = "dashboard" | "journal" | "backtest" | "live-risk" | "trading-plan" | "news" | "strategy" | "analytics" | "accounts" | "equity-engine" | "settings";

interface Props {
  active: Page;
  onNavigate: (p: Page) => void;
}

const items: { id: Page; label: string; icon: string }[] = [
  { id: "dashboard", label: "Dashboard", icon: "▦" },
  { id: "journal",   label: "Journal",   icon: "▤" },
  { id: "backtest", label: "Backtest", icon: "▷" },
  { id: "live-risk", label: "Live Risk", icon: "◐" },
  { id: "trading-plan", label: "Trading Plan", icon: "✎" },
  { id: "news",      label: "News",      icon: "📰" },
  { id: "strategy",  label: "Strategy",  icon: "◈" },
  { id: "analytics", label: "Analytics", icon: "▥" },
  { id: "accounts",  label: "Accounts",  icon: "◉" },
  { id: "equity-engine", label: "Equity Engine", icon: "⚖" },
  { id: "settings",  label: "Settings",  icon: "⚙" },
];

export function Sidebar({ active, onNavigate }: Props) {
  return (
    <aside className="sidebar">
      <div className="sidebar__brand">
        <span className="sidebar__logo">◆</span>
        <span className="sidebar__title">Trading Manager</span>
      </div>

      <nav className="sidebar__nav">
        {items.map((it) => (
          <button
            key={it.id}
            className={`sidebar__item ${active === it.id ? "is-active" : ""}`}
            onClick={() => onNavigate(it.id)}
          >
            <span className="sidebar__icon">{it.icon}</span>
            <span className="sidebar__label">{it.label}</span>
          </button>
        ))}
      </nav>

      <div className="sidebar__footer">
        <div className="sidebar__version">
          <span className="mono">v0.1.0</span>
          <span className="sidebar__mode">local · offline</span>
        </div>
      </div>
    </aside>
  );
}
