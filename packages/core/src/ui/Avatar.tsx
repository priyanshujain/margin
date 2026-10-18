import "./Avatar.css";

export type AvatarSize = "xs" | "sm" | "md" | "lg";

export interface AvatarProps {
  name: string;
  address?: string;
  brand?: boolean;
  hue?: number;
  size?: AvatarSize;
}

export function avatarHue(key: string): number {
  let h = 0;
  for (let i = 0; i < key.length; i++) h = (h * 31 + key.charCodeAt(i)) >>> 0;
  return (h % 8) + 1;
}

export function avatarInitials(name: string, brand?: boolean): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return "?";
  if (brand) {
    const w = words[0];
    return (w[0] ?? "").toUpperCase() + (w[1] ?? "").toLowerCase();
  }
  if (words.length === 1) return (words[0][0] ?? "").toUpperCase();
  return ((words[0][0] ?? "") + (words[words.length - 1][0] ?? "")).toUpperCase();
}

export function Avatar({ name, address, brand, hue: chosen, size = "md" }: AvatarProps) {
  const hue = chosen ?? avatarHue(address ?? name);
  return (
    <span
      className="avatar"
      data-size={size}
      data-hue={brand ? undefined : hue}
      data-brand={brand ? "" : undefined}
      aria-hidden="true"
    >
      {avatarInitials(name, brand)}
    </span>
  );
}

export interface AvatarStackProps {
  people: { name: string; address?: string; brand?: boolean }[];
  size?: AvatarSize;
  max?: number;
}

export function AvatarStack({ people, size = "sm", max = 4 }: AvatarStackProps) {
  const faces = people.length > max ? people.slice(0, max) : people;
  const rest = people.length - faces.length;
  return (
    <span className="avatar-stack">
      {faces.map((p) => (
        <Avatar key={p.address ?? p.name} name={p.name} address={p.address} brand={p.brand} size={size} />
      ))}
      {rest > 0 ? (
        <span className="avatar" data-size={size} data-more="" aria-hidden="true">
          {`+${rest}`}
        </span>
      ) : null}
    </span>
  );
}
