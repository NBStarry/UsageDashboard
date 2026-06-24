// Theme utilities: color thresholds, time formatting, reset countdown.

// barColor returns a hex color for a usage percentage.
// >=90 → red (#F85149), >=75 → yellow (#D29922), else green (#3FB950).
export function barColor(pct: number): string {
  if (pct >= 90) return '#F85149';
  if (pct >= 75) return '#D29922';
  return '#3FB950';
}

// hm formats a Date (or ISO string) as HH:mm (24-hour, zero-padded).
// 防御性:date 为空或非法时返回 "--:--" 而非抛错——渲染中一旦抛错会中断
// Svelte 的 effect flush,使整个组件后续不再更新(见历史 fetchedAt 序列化 bug)。
export function hm(date: Date | string | null | undefined): string {
  if (date == null) return '--:--';
  const d = typeof date === 'string' ? new Date(date) : date;
  if (Number.isNaN(d.getTime())) return '--:--';
  const h = String(d.getHours()).padStart(2, '0');
  const m = String(d.getMinutes()).padStart(2, '0');
  return `${h}:${m}`;
}

// resetCountdown formats the time remaining until an ISO date string (or Date) as
// "重置 Xh Ym 后" when at least 1 hour remains, or "重置 Ym 后" for under 1 hour.
// Returns an empty string if the date is in the past or falsy.
export function resetCountdown(isoOrDate: string | Date | null | undefined): string {
  if (!isoOrDate) return '';
  const target = typeof isoOrDate === 'string' ? new Date(isoOrDate) : isoOrDate;
  const diffMs = target.getTime() - Date.now();
  if (diffMs <= 0) return '';
  const totalMinutes = Math.floor(diffMs / 60_000);
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours >= 1) {
    return `重置 ${hours}h ${minutes}m 后`;
  }
  return `重置 ${minutes}m 后`;
}
