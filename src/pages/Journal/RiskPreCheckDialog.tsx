import { PreEvalResult } from "../../lib/equityApi";

interface Props {
  result: PreEvalResult;
  userSize: string;
  onAdjust: (newSize: string) => void;
  onContinue: () => void;
  onCancel: () => void;
}

export function RiskPreCheckDialog({
  result,
  userSize,
  onAdjust,
  onContinue,
  onCancel,
}: Props) {
  const verdict = result.verdict;
  const allowed = result.allowed_size;

  const toneClass =
    verdict === "ok"
      ? "is-ok"
      : verdict === "warn"
      ? "is-warn"
      : "is-block"; // block or reject

  const title =
    verdict === "reject"
      ? "Trade not allowed"
      : verdict === "block"
      ? "Size far above limit"
      : verdict === "warn"
      ? "Size above limit"
      : "Size within limit";

  const icon =
    verdict === "reject" ? "🚫" : verdict === "block" ? "⛔" : verdict === "warn" ? "⚠" : "✅";

  return (
    <div className="risk-backdrop" onClick={onCancel}>
      <div
        className={`risk-dialog ${toneClass}`}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="risk-dialog__header">
          <span className="risk-dialog__icon">{icon}</span>
          <h3 className="risk-dialog__title">{title}</h3>
        </div>

        <div className="risk-dialog__body">
          <p className="risk-dialog__reason">{result.reason}</p>

          <div className="risk-dialog__grid">
            <div className="risk-dialog__item">
              <div className="risk-dialog__label">Your size</div>
              <div className="risk-dialog__value mono">{userSize} lots</div>
            </div>
            <div className="risk-dialog__item">
              <div className="risk-dialog__label">Allowed size</div>
              <div className="risk-dialog__value mono">
                {allowed !== null ? `${parseFloat(allowed).toFixed(2)} lots` : "—"}
              </div>
            </div>
            {result.ratio && (
              <div className="risk-dialog__item">
                <div className="risk-dialog__label">Ratio</div>
                <div className="risk-dialog__value mono">
                  {parseFloat(result.ratio).toFixed(2)}×
                </div>
              </div>
            )}
          </div>

          {verdict === "reject" && (
            <div className="risk-dialog__note risk-dialog__note--error">
              The Equity Engine has rejected this trade — you cannot save it.
              Check the reason above.
            </div>
          )}

          {verdict === "block" && (
            <div className="risk-dialog__note risk-dialog__note--error">
              This size is heavily above the limit. Strongly recommended to
              adjust before saving.
            </div>
          )}

          {verdict === "warn" && (
            <div className="risk-dialog__note risk-dialog__note--warn">
              This size is above the allowed risk. Continuing will increase
              your exposure beyond the configured limits.
            </div>
          )}
        </div>

        <div className="risk-dialog__footer">
          {verdict === "reject" && (
            <>
              <button className="btn btn--ghost" onClick={onCancel}>
                Close
              </button>
            </>
          )}

          {verdict === "warn" && (
            <>
              <button className="btn btn--ghost" onClick={onCancel}>
                Cancel
              </button>
              {allowed !== null && (
                <button
                  className="btn btn--primary"
                  onClick={() => onAdjust(parseFloat(allowed).toFixed(2))}
                >
                  Adjust to {parseFloat(allowed).toFixed(2)} lots
                </button>
              )}
              <button className="btn btn--ghost" onClick={onContinue}>
                Continue anyway
              </button>
            </>
          )}

          {verdict === "block" && (
            <>
              <button className="btn btn--ghost" onClick={onCancel}>
                Cancel
              </button>
              {allowed !== null && (
                <button
                  className="btn btn--primary"
                  onClick={() => onAdjust(parseFloat(allowed).toFixed(2))}
                >
                  Adjust to {parseFloat(allowed).toFixed(2)} lots
                </button>
              )}
              <button className="btn btn--ghost" onClick={onContinue}>
                Override & save
              </button>
            </>
          )}

          {verdict === "ok" && (
            <button className="btn btn--primary" onClick={onContinue}>
              Save
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
