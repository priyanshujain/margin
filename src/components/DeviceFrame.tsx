import type { CSSProperties, ReactNode } from "react";
import type { Device } from "../devices";
import type { Book } from "../model/book";
import { CoverArt } from "./CoverArt";

interface DeviceFrameProps {
  device: Device;
  eyebrow?: ReactNode;
  title?: ReactNode;
  html?: string;
  cover?: Book;
}

function frameVars(device: Device): CSSProperties {
  return {
    "--dv-bg": device.bg,
    "--dv-ink": device.ink,
    "--dv-fs": device.fontSize,
    "--dv-lh": device.lineHeight,
    "--dv-mx": device.marginX,
    "--dv-my": device.marginY,
    "--dv-bezel": device.bezel,
    "--dv-bezel-w": device.bezelWidth,
    "--dv-radius": device.radius,
    "--dv-aspect": device.aspect,
  } as CSSProperties;
}

function StatusBar() {
  return (
    <div className="device-statusbar">
      <span className="device-clock">9:41</span>
      <span className="device-batt" />
    </div>
  );
}

export function DeviceFrame({ device, eyebrow, title, html, cover }: DeviceFrameProps) {
  return (
    <div className="device-stage">
      <div className="device-frame" data-kind={device.kind} style={frameVars(device)}>
        <div className="device-screen">
          {device.kind === "tablet" && <StatusBar />}
          <div className="device-content">
            {cover ? (
              <CoverArt book={cover} />
            ) : (
              <>
                <div className="device-opener">
                  {eyebrow && <div className="device-eyebrow">{eyebrow}</div>}
                  {title && <div className="device-title">{title}</div>}
                </div>
                <div className="device-body" dangerouslySetInnerHTML={{ __html: html ?? "" }} />
              </>
            )}
          </div>
          {device.kind === "eink" && (
            <div className="device-footer">
              <span className="device-foot-title">{cover ? device.label : title}</span>
              <span className="device-batt" />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
