import { HelpTip } from "./HelpTip";
import "./FieldWithHelp.css";

interface Props {
  label: string;
  help?: { title?: string; body: string };
  children: React.ReactNode;
  align?: "start" | "end" | "center";
}

/**
 * A form row with an inline "?" help trigger positioned at the right edge
 * of the input — not next to the label. The input fills the row and the
 * help button overlays its right padding.
 */
export function FieldWithHelp({ label, help, children, align = "end" }: Props) {
  return (
    <label className="fwh">
      <span className="fwh__label">{label}</span>
      <div className="fwh__control">
        {children}
        {help && (
          <span className="fwh__help">
            <HelpTip title={help.title} align={align}>
              {help.body}
            </HelpTip>
          </span>
        )}
      </div>
    </label>
  );
}
