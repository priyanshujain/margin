import "./Key.css";

export interface KeyProps {
  children: string;
  size?: "sm" | "md";
}

export function Key({ children, size = "md" }: KeyProps) {
  return (
    <kbd className="key" data-size={size}>
      {children}
    </kbd>
  );
}
