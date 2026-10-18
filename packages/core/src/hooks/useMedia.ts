import { useEffect, useState } from "react";

export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(() => window.matchMedia(query).matches);
  useEffect(() => {
    const mq = window.matchMedia(query);
    const onChange = () => setMatches(mq.matches);
    onChange();
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [query]);
  return matches;
}

export const COMPACT_QUERY = "(max-width: 899px)";
export const PHONE_QUERY = "(max-width: 640px)";
export const TOUCH_QUERY = "(pointer: coarse)";

export function usePhone(): boolean {
  const phone = useMediaQuery(PHONE_QUERY);
  useEffect(() => {
    document.documentElement.toggleAttribute("data-phone", phone);
  }, [phone]);
  return phone;
}

export function useTouch(): boolean {
  const touch = useMediaQuery(TOUCH_QUERY);
  useEffect(() => {
    document.documentElement.toggleAttribute("data-touch", touch);
  }, [touch]);
  return touch;
}

export function useCompact(): boolean {
  const compact = useMediaQuery(COMPACT_QUERY);
  useEffect(() => {
    document.documentElement.toggleAttribute("data-compact", compact);
  }, [compact]);
  return compact;
}
