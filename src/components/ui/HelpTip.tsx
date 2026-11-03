import { useEffect, useRef, useState } from "react";
import "./HelpTip.css";

interface Props {
  title?: string;
  children: React.ReactNode;
  align?: "start" | "end" | "center";
}

export function HelpTip({ title, children, align = "start" }: Props) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    if (!open) return;
    function onDoc(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <span className="helptip" ref={ref}>
      <button
        type="button"
        className="helptip__trigger"
        aria-label="Help"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        ?
      </button>
      {open && (
        <div className={`helptip__pop helptip__pop--${align} pop-in`} role="tooltip">
          {title && <div className="helptip__title">{title}</div>}
          <div className="helptip__body">{children}</div>
        </div>
      )}
    </span>
  );
}
