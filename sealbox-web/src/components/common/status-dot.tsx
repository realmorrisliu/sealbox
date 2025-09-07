type Tone = "success" | "warning" | "info" | "destructive" | "muted";

export function StatusDot({
  color,
  tone = "muted",
  title,
}: {
  color?: string;
  tone?: Tone;
  title?: string;
}) {
  const toneClass =
    tone === "success"
      ? "bg-success"
      : tone === "warning"
        ? "bg-warning"
        : tone === "info"
          ? "bg-info"
          : tone === "destructive"
            ? "bg-destructive"
            : "bg-muted-foreground/40";
  return (
    <span
      className={`inline-block h-2 w-2 rounded-full ${color || toneClass}`}
      title={title}
      aria-label={title}
    />
  );
}
