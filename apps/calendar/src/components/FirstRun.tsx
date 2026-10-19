// What the app says with no calendar at all. A first launch never gets here: it opens on a calendar
// on this device. Deleting every calendar does, and then there is nothing to draw on, so the two
// ways to get one back cover the grid rather than an empty axis that reads as a failed load.

import { runCommand } from "@margin/core/keys";
import { useAccounts } from "../store/useAccounts";

export function FirstRun() {
  const loaded = useAccounts((s) => s.loaded);
  const calendars = useAccounts((s) => s.calendars);
  const createCalendar = useAccounts((s) => s.createCalendar);

  if (!loaded || calendars.length > 0) return null;

  return (
    <div className="first-run">
      <div className="first-run-text">
        <h2 className="first-run-title">No calendars</h2>
        <p className="first-run-note">
          Make one on this device, or connect an account and its calendars show up here.
        </p>
        <div className="first-run-actions">
          <button
            type="button"
            className="button"
            data-size="md"
            data-variant="primary"
            onClick={() => void createCalendar("Calendar")}
          >
            New calendar
          </button>
          <button
            type="button"
            className="button"
            data-size="md"
            onClick={() => runCommand("accounts")}
          >
            Add an account
          </button>
        </div>
      </div>
    </div>
  );
}

export default FirstRun;
