import { invoke } from "@tauri-apps/api/core";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const isMobileOs =
  typeof navigator !== "undefined" &&
  (/android|iphone|ipod/i.test(navigator.userAgent) ||
    (/ipad|macintosh/i.test(navigator.userAgent) && navigator.maxTouchPoints > 1));

export const isDesktop = isTauri && !isMobileOs;

export const isMacDesktop =
  isDesktop && typeof navigator !== "undefined" && /mac/i.test(navigator.userAgent);

export const live = (): boolean => isTauri || import.meta.env.DEV;

export type MockCall = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export interface CallOptions {
  mock?: () => Promise<MockCall>;
  logCommand?: string | null;
}

export type Call = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createCall({ mock, logCommand = "log_note" }: CallOptions = {}): Call {
  return <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
    if (import.meta.env.DEV && !isTauri && mock) {
      return mock().then((mockCall) => mockCall<T>(command, args));
    }
    return invoke<T>(command, args).catch((e: unknown) => {
      if (logCommand && command !== logCommand) {
        void invoke(logCommand, { who: "ui", line: `${command}: ${String(e)}` }).catch(() => {});
      }
      throw e;
    });
  };
}
