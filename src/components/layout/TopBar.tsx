import "./TopBar.css";

interface Props {
  title: string;
  subtitle?: string;
}

export function TopBar({ title, subtitle }: Props) {
  return (
    <header className="topbar">
      <div>
        <h1 className="topbar__title">{title}</h1>
        {subtitle && <p className="topbar__subtitle">{subtitle}</p>}
      </div>
      <div className="topbar__right">
        <span className="topbar__pill">Local · Offline</span>
      </div>
    </header>
  );
}
