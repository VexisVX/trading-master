import { useEffect } from "react";
import "./ConfirmDialog.css";

interface Props {
  title: string;
  body: string;
  confirmLabel?: string;
  cancelLabel?: string;
  tone?: "danger" | "default";
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  title, body,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  tone = "danger",
  onConfirm, onCancel,
}: Props) {
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onCancel();
      if (e.key === "Enter") onConfirm();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onCancel, onConfirm]);

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal confirm-dialog" onClick={(e) => e.stopPropagation()}>
        <h3>{title}</h3>
        <p className="confirm-dialog__body">{body}</p>
        <div className="modal__actions">
          <button className="btn btn--ghost" onClick={onCancel}>{cancelLabel}</button>
          <button
            className={`btn ${tone === "danger" ? "btn--danger" : "btn--primary"}`}
            onClick={onConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
