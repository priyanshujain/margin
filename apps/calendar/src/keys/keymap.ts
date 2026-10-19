// margin-core's keymap, plus the two frames only Calendar knows about.
//
// Every overlay is summoned through `useOverlays`, and Settings through `useSettingsScreen`, so the
// frame is synced here rather than asked of each panel: a panel that forgot to push one would leave the grid's keys live under it. The grid's
// in-place create input is the other: while a draft is open it pushes `editor`, which shadows the
// view keymap the same way and leaves the palette reachable.

import { useEffect } from "react";
import { installKeymap, pushContext } from "@margin/core/keys";
import { useGrid } from "../components/GridStore";
import { useOverlays } from "../store/useOverlays";
import { useSettingsScreen } from "../store/useSettingsScreen";
import { useCalendarsPopover } from "../components/CalendarsPopover";
import "./bindings";
import "./commands";

function frameWhile(context: string): (on: boolean) => void {
  let pop: (() => void) | null = null;
  return (on) => {
    if (on && !pop) pop = pushContext(context);
    else if (!on && pop) {
      pop();
      pop = null;
    }
  };
}

export function useKeymap(): void {
  useEffect(() => {
    const overlay = frameWhile("overlay");
    const editor = frameWhile("editor");
    const covered = () => {
      const pop = useCalendarsPopover.getState();
      return (
        useOverlays.getState().open !== null ||
        useSettingsScreen.getState().open ||
        pop.open ||
        pop.action !== null
      );
    };
    overlay(covered());
    editor(useGrid.getState().draft !== null);
    const stopOverlays = useOverlays.subscribe(() => overlay(covered()));
    const stopSettings = useSettingsScreen.subscribe(() => overlay(covered()));
    const stopPopover = useCalendarsPopover.subscribe(() => overlay(covered()));
    const stopGrid = useGrid.subscribe((s) => editor(s.draft !== null));
    const uninstall = installKeymap();
    return () => {
      uninstall();
      stopOverlays();
      stopSettings();
      stopPopover();
      stopGrid();
      overlay(false);
      editor(false);
    };
  }, []);
}
