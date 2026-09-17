export function Sparkline({
  values,
  className,
}: {
  values: number[];
  className?: string;
}) {
  const w = 88;
  const h = 28;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const pts = values
    .map((v, i) => {
      const x = (i / (values.length - 1)) * w;
      const y = h - 2 - ((v - min) / span) * (h - 4);
      return `${x},${y}`;
    })
    .join(" ");
  const up = values[values.length - 1] >= values[0];
  return (
    <svg
      viewBox={`0 0 ${w} ${h}`}
      width={w}
      height={h}
      className={className}
      aria-hidden
    >
      <polyline
        fill="none"
        stroke={up ? "var(--color-gain)" : "var(--color-loss)"}
        strokeWidth="1.5"
        points={pts}
      />
    </svg>
  );
}
