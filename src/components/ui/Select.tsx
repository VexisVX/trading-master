import { useEffect, useRef, useState } from "react";
import "./Select.css";

export interface SelectOption<T extends string> {
  value: T;
  label: string;
}

interface Props<T extends string> {
  value: T;
  options: SelectOption<T>[];
  onChange: (v: T) => void;
  placeholder?: string;
}

export function Select<T extends string>({
  value, options, onChange, placeholder,
}: Props<T>) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onDoc(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    if (open) document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const current = options.find((o) => o.value === value);

  return (
    <div className="select" ref={ref}>
      <button
        type="button"
        className={`select__trigger ${open ? "is-open" : ""}`}
        onClick={() => setOpen((v) => !v)}
      >
        <span className={current ? "" : "select__placeholder"}>
          {current?.label ?? placeholder ?? "Select…"}
        </span>
        <span className="select__chevron">▾</span>
      </button>

      {open && (
        <div className="select__menu" role="listbox">
          {options.map((o) => (
            <button
              key={o.value}
              type="button"
              className={`select__item ${o.value === value ? "is-active" : ""}`}
              onClick={() => { onChange(o.value); setOpen(false); }}
            >
              {o.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
