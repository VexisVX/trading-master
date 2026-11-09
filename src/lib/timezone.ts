// Timezone helpers for News / Calendar display.
// Separate from app-wide timezone settings.

export interface TimezoneOption {
  id: string;      // IANA tz
  label: string;   // human label
  short: string;   // UTC offset short label
}

export const NEWS_TIMEZONES: TimezoneOption[] = [
  { id: "Asia/Tehran",       label: "Iran · Tehran",       short: "IRST" },
  { id: "America/New_York",  label: "New York",            short: "ET" },
  { id: "Europe/London",     label: "London",              short: "GMT/BST" },
  { id: "Europe/Berlin",     label: "Frankfurt",           short: "CET" },
  { id: "Asia/Tokyo",        label: "Tokyo",               short: "JST" },
  { id: "Asia/Dubai",        label: "Dubai",               short: "GST" },
  { id: "Asia/Hong_Kong",    label: "Hong Kong",           short: "HKT" },
  { id: "Australia/Sydney",  label: "Sydney",              short: "AEDT" },
  { id: "UTC",               label: "UTC",                 short: "UTC" },
];

/**
 * Format a timestamp in the given timezone.
 * If timezone is empty, uses the local system timezone.
 */
export function formatInTz(
  iso: string,
  tz: string,
  opts: { hour?: "2-digit" | "numeric"; minute?: "2-digit" | "numeric"; weekday?: "long" | "short"; month?: "long" | "short"; day?: "numeric" | "2-digit"; year?: "numeric" } = {}
): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;

  const options: Intl.DateTimeFormatOptions = {
    timeZone: tz || undefined,
    hour12: false,
    hour: opts.hour ?? "2-digit",
    minute: opts.minute ?? "2-digit",
    weekday: opts.weekday,
    month: opts.month,
    day: opts.day,
    year: opts.year,
  };

  try {
    return new Intl.DateTimeFormat(undefined, options).format(d);
  } catch {
    return d.toISOString();
  }
}

export function formatTimeInTz(iso: string, tz: string): string {
  return formatInTz(iso, tz, { hour: "2-digit", minute: "2-digit" });
}

export function formatDayInTz(iso: string, tz: string): string {
  // iso may be YYYY-MM-DD (calendar) or full ISO
  const dateStr = iso.length === 10 ? iso + "T12:00:00Z" : iso;
  return formatInTz(dateStr, tz, { weekday: "long", month: "short", day: "numeric" });
}

export function formatShortDateInTz(iso: string, tz: string): string {
  const dateStr = iso.length === 10 ? iso + "T12:00:00Z" : iso;
  return formatInTz(dateStr, tz, { month: "short", day: "numeric" });
}

/**
 * Get the current timezone offset label (e.g., "UTC+3:30")
 */
export function getTzOffset(tz: string): string {
  try {
    const now = new Date();
    const formatter = new Intl.DateTimeFormat("en-US", {
      timeZone: tz,
      timeZoneName: "short",
    });
    const parts = formatter.formatToParts(now);
    const tzName = parts.find((p) => p.type === "timeZoneName");
    return tzName?.value || "";
  } catch {
    return "";
  }
}
