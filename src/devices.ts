export type DeviceId = "kindle" | "kobo" | "ipad" | "iphone" | "android";
export type PreviewMode = "print" | DeviceId;
export type DeviceKind = "eink" | "phone" | "tablet";

export interface Device {
  id: DeviceId;
  label: string;
  kind: DeviceKind;
  bg: string;
  ink: string;
  fontSize: number;
  lineHeight: number;
  marginX: number;
  marginY: number;
  aspect: number;
  bezel: string;
  bezelWidth: number;
  radius: number;
}

export const DEVICES: Device[] = [
  {
    id: "kindle",
    label: "Kindle Paperwhite",
    kind: "eink",
    bg: "#f4f3ee",
    ink: "#2b2b2b",
    fontSize: 16,
    lineHeight: 1.55,
    marginX: 34,
    marginY: 40,
    aspect: 1240 / 1680,
    bezel: "#1c1c1c",
    bezelWidth: 18,
    radius: 14,
  },
  {
    id: "kobo",
    label: "Kobo",
    kind: "eink",
    bg: "#f6f5f0",
    ink: "#262626",
    fontSize: 16,
    lineHeight: 1.6,
    marginX: 34,
    marginY: 40,
    aspect: 1264 / 1680,
    bezel: "#2a2a2a",
    bezelWidth: 16,
    radius: 16,
  },
  {
    id: "ipad",
    label: "iPad",
    kind: "tablet",
    bg: "#fbfaf7",
    ink: "#1d1d1f",
    fontSize: 18,
    lineHeight: 1.65,
    marginX: 56,
    marginY: 56,
    aspect: 1640 / 2360,
    bezel: "#3a3a3c",
    bezelWidth: 20,
    radius: 28,
  },
  {
    id: "iphone",
    label: "iPhone",
    kind: "phone",
    bg: "#ffffff",
    ink: "#1d1d1f",
    fontSize: 17,
    lineHeight: 1.6,
    marginX: 26,
    marginY: 30,
    aspect: 1179 / 2556,
    bezel: "#1a1a1c",
    bezelWidth: 12,
    radius: 44,
  },
  {
    id: "android",
    label: "Android phone",
    kind: "phone",
    bg: "#ffffff",
    ink: "#202124",
    fontSize: 16,
    lineHeight: 1.6,
    marginX: 24,
    marginY: 28,
    aspect: 1080 / 2400,
    bezel: "#0f0f0f",
    bezelWidth: 11,
    radius: 36,
  },
];

const DEVICE_MAP = new Map<string, Device>(DEVICES.map((d) => [d.id, d]));

const KEY = "margin-preview-mode";

export function findDevice(id: string): Device | undefined {
  return DEVICE_MAP.get(id);
}

export function initialPreviewMode(): PreviewMode {
  const saved = localStorage.getItem(KEY);
  if (saved === "print" || (saved && DEVICE_MAP.has(saved))) return saved as PreviewMode;
  return "print";
}

export function persistPreviewMode(mode: PreviewMode) {
  localStorage.setItem(KEY, mode);
}
