const DEFAULT_LOCALE = "zh-CN";

function normalizeDateInput(value?: string | null): string | null {
  const trimmed = value?.trim();
  if (!trimmed) {
    return null;
  }

  if (/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}/.test(trimmed)) {
    return trimmed.replace(" ", "T");
  }

  return trimmed;
}

export function parseDateValue(value?: string | null): Date | null {
  const normalized = normalizeDateInput(value);
  if (!normalized) {
    return null;
  }

  const parsed = new Date(normalized);
  if (!Number.isNaN(parsed.getTime())) {
    return parsed;
  }

  const withExplicitOffset = /[zZ]|[+-]\d{2}:\d{2}$/.test(normalized)
    ? normalized
    : `${normalized}+08:00`;
  const reparsed = new Date(withExplicitOffset);
  return Number.isNaN(reparsed.getTime()) ? null : reparsed;
}

export function formatDateTime(
  value?: string | null,
  locale = DEFAULT_LOCALE,
  fallback = "时间未知",
) {
  const date = parseDateValue(value);
  if (!date) {
    return fallback;
  }

  return date.toLocaleString(locale, { hour12: false });
}

export function formatClockTime(
  value?: string | null,
  locale = DEFAULT_LOCALE,
  fallback = "--:--",
) {
  const date = parseDateValue(value);
  if (!date) {
    return fallback;
  }

  return date.toLocaleTimeString(locale, {
    hour: "2-digit",
    minute: "2-digit",
  });
}
