import { useEffect, useMemo, useRef, useState } from "react";
import { listSetupTags, SetupTagCount } from "../../lib/backtestApi";

interface Props {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}

export function SetupInput({ value, onChange, placeholder = "e.g. FVG+OB" }: Props) {
  const [all, setAll] = useState<SetupTagCount[]>([]);
  const [open, setOpen] = useState(false);
  const [highlight, setHighlight] = useState(0);
  const wrapRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    listSetupTags()
      .then(setAll)
      .catch(() => setAll([]));
  }, []);

  useEffect(() => {
    function onDoc(e: MouseEvent) {
      if (!wrapRef.current?.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);

  const suggestions = useMemo(() => {
    const q = value.trim().toLowerCase();
    const pool = q
      ? all.filter((t) => t.tag.toLowerCase().includes(q) && t.tag.toLowerCase() !== q)
      : all;
    return pool.slice(0, 8);
  }, [value, all]);

  function choose(tag: string) {
    onChange(tag);
    setOpen(false);
    setHighlight(0);
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (!open || suggestions.length === 0) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlight((h) => (h + 1) % suggestions.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlight((h) => (h - 1 + suggestions.length) % suggestions.length);
    } else if (e.key === "Enter" || e.key === "Tab") {
      if (suggestions[highlight]) {
        e.preventDefault();
        choose(suggestions[highlight].tag);
      }
    } else if (e.key === "Escape") {
      setOpen(false);
    }
  }

  return (
    <div className="bt-setup" ref={wrapRef}>
      <input
        value={value}
        onChange={(e) => {
          onChange(e.target.value);
          setOpen(true);
          setHighlight(0);
        }}
        onFocus={() => setOpen(true)}
        onKeyDown={onKeyDown}
        placeholder={placeholder}
        autoComplete="off"
      />
      {open && suggestions.length > 0 && (
        <ul className="bt-setup__menu">
          {suggestions.map((s, i) => (
            <li
              key={s.tag}
              className={`bt-setup__item ${i === highlight ? "is-hi" : ""}`}
              onMouseDown={(e) => {
                e.preventDefault();
                choose(s.tag);
              }}
              onMouseEnter={() => setHighlight(i)}
            >
              <span className="bt-setup__tag">{s.tag}</span>
              <span className="bt-setup__count">{s.count}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
