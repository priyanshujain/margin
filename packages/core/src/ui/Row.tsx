import { memo } from "react";

import { Avatar } from "./Avatar";
import { Icon } from "./Icon";
import { CHECK } from "../icons";
import "./Row.css";

export interface RowProps {
  sender: string;
  address?: string;
  time: string;
  subject: string;
  snippet?: string;
  count?: number;
  note?: string;
  brand?: boolean;
  accountHue?: number;
  unread?: boolean;
  selected?: boolean;
  selecting?: boolean;
  checked?: boolean;
  mark?: string;
  markTitle?: string;
  onClick?: () => void;
  onToggleCheck?: () => void;
}

function RowBase({
  sender,
  address,
  time,
  subject,
  snippet,
  count,
  note,
  brand,
  accountHue,
  unread,
  selected,
  selecting,
  checked,
  mark,
  markTitle,
  onClick,
  onToggleCheck,
}: RowProps) {
  return (
    <div
      className="row"
      data-new={unread ? "" : undefined}
      data-selected={selected ? "" : undefined}
      data-selecting={selecting ? "" : undefined}
      data-checked={checked ? "" : undefined}
      data-noted={note ? "" : undefined}
      onClick={onClick}
    >
      {accountHue ? <span className="row-account" data-hue={accountHue} /> : null}

      <div className="row-gutter">
        {selecting ? (
          <button
            type="button"
            className="row-check"
            role="checkbox"
            aria-checked={Boolean(checked)}
            aria-label={`Select ${sender}`}
            onClick={(e) => {
              e.stopPropagation();
              onToggleCheck?.();
            }}
          >
            {checked ? <Icon d={CHECK} size={10} /> : null}
          </button>
        ) : null}
      </div>

      <Avatar name={sender} address={address} brand={brand} />

      <div className="row-main">
        <div className="row-top">
          <span className="row-sender">{sender}</span>
          {count && count > 1 ? <span className="row-count">{count}</span> : null}
          {mark ? (
            <span className="row-mark" title={markTitle}>
              <Icon d={mark} size={13} />
            </span>
          ) : null}
          <span className="row-time">{time}</span>
        </div>
        <div className="row-bottom">
          <span className="row-subject">{subject}</span>
          {snippet ? <span className="row-snippet">{snippet}</span> : null}
        </div>
      </div>

      {note ? <span className="row-note">{note}</span> : null}
    </div>
  );
}

function draws(a: RowProps, b: RowProps): boolean {
  return (
    a.sender === b.sender &&
    a.address === b.address &&
    a.time === b.time &&
    a.subject === b.subject &&
    a.snippet === b.snippet &&
    a.count === b.count &&
    a.note === b.note &&
    a.brand === b.brand &&
    a.accountHue === b.accountHue &&
    a.unread === b.unread &&
    a.selected === b.selected &&
    a.selecting === b.selecting &&
    a.checked === b.checked &&
    a.mark === b.mark &&
    a.markTitle === b.markTitle
  );
}

export const Row = memo(RowBase, draws);
