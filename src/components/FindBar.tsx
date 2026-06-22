import { useEffect, useMemo, useRef, useState } from "react";
import type { Editor as TiptapEditor } from "@tiptap/react";
import { buildRegex, getSearchState } from "../editor/search";
import { countMatches, replaceInContent } from "../search-book";
import { useBook } from "../store/useBook";
import { Icon } from "./Icon";

interface FindBarProps {
  editor: TiptapEditor | null;
  open: boolean;
  initialExpanded: boolean;
  onClose: () => void;
}

type Scope = "chapter" | "book";
type PendingNav = "first" | "last" | null;

export function FindBar({ editor, open, initialExpanded, onClose }: FindBarProps) {
  const book = useBook((s) => s.book);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const setActiveChapter = useBook((s) => s.setActiveChapter);
  const setChapterContent = useBook((s) => s.setChapterContent);
  const setNotice = useBook((s) => s.setNotice);

  const [query, setQuery] = useState("");
  const [replacement, setReplacement] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [expanded, setExpanded] = useState(initialExpanded);
  const [scope, setScope] = useState<Scope>("chapter");
  const [active, setActive] = useState({ count: 0, current0: 0 });
  const [pendingNav, setPendingNav] = useState<PendingNav>(null);
  const findInputRef = useRef<HTMLInputElement>(null);

  const chapters = useMemo(() => book?.chapters ?? [], [book]);
  const activeIdx = chapters.findIndex((c) => c.id === activeChapterId);
  const options = { caseSensitive, wholeWord };

  const jsonCounts = useMemo(() => {
    const regex = query ? buildRegex(query, options) : null;
    if (!regex) return chapters.map(() => 0);
    return chapters.map((c) => countMatches(c.content, regex));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chapters, query, caseSensitive, wholeWord]);

  useEffect(() => {
    if (open) setExpanded(initialExpanded);
  }, [open, initialExpanded]);

  useEffect(() => {
    if (!editor) return;
    const update = () => {
      const s = getSearchState(editor.state);
      setActive({ count: s?.matches.length ?? 0, current0: s?.current ?? 0 });
    };
    editor.on("transaction", update);
    update();
    return () => {
      editor.off("transaction", update);
    };
  }, [editor]);

  useEffect(() => {
    if (!editor) return;
    if (open) {
      editor.commands.setSearch(query, { caseSensitive, wholeWord });
    } else {
      editor.commands.clearSearch();
    }
  }, [editor, open, query, caseSensitive, wholeWord]);

  useEffect(() => {
    if (open) {
      const input = findInputRef.current;
      input?.focus();
      input?.select();
    }
  }, [open]);

  useEffect(() => {
    if (!editor || !pendingNav) return;
    let done = false;
    const go = () => {
      if (done) return;
      done = true;
      const s = getSearchState(editor.state);
      if (s && s.matches.length) editor.commands.goToMatch(pendingNav === "first" ? 0 : s.matches.length - 1);
      setPendingNav(null);
    };
    editor.on("transaction", go);
    const raf = requestAnimationFrame(go);
    return () => {
      editor.off("transaction", go);
      cancelAnimationFrame(raf);
    };
  }, [editor, pendingNav, activeChapterId]);

  if (!open || !editor) return null;

  const before = jsonCounts.slice(0, Math.max(0, activeIdx)).reduce((a, b) => a + b, 0);
  const after = jsonCounts.slice(activeIdx + 1).reduce((a, b) => a + b, 0);
  const total = before + active.count + after;
  const globalCurrent = before + active.current0 + 1;
  const otherHasMatches = chapters.some((c, i) => c.id !== activeChapterId && jsonCounts[i] > 0);

  const countLabel = !query
    ? ""
    : total === 0
    ? "No results"
    : active.count === 0
    ? `${total} found`
    : `${globalCurrent} of ${total}`;

  const nextChapterWith = (dir: 1 | -1): string | null => {
    const n = chapters.length;
    if (!n || activeIdx < 0) return null;
    for (let step = 1; step <= n; step++) {
      const i = (((activeIdx + dir * step) % n) + n) % n;
      if (chapters[i].id !== activeChapterId && jsonCounts[i] > 0) return chapters[i].id;
    }
    return null;
  };

  const next = () => {
    const s = getSearchState(editor.state);
    const count = s?.matches.length ?? 0;
    const cur = s?.current ?? 0;
    if (count > 0 && cur < count - 1) {
      editor.commands.findNext();
      return;
    }
    if (otherHasMatches) {
      const id = nextChapterWith(1);
      if (id) {
        setPendingNav("first");
        setActiveChapter(id);
        return;
      }
    }
    editor.commands.findNext();
  };

  const prev = () => {
    const s = getSearchState(editor.state);
    const count = s?.matches.length ?? 0;
    const cur = s?.current ?? 0;
    if (count > 0 && cur > 0) {
      editor.commands.findPrev();
      return;
    }
    if (otherHasMatches) {
      const id = nextChapterWith(-1);
      if (id) {
        setPendingNav("last");
        setActiveChapter(id);
        return;
      }
    }
    editor.commands.findPrev();
  };

  const onFindKey = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      e.shiftKey ? prev() : next();
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  };

  const replaceOne = () => {
    editor.commands.replaceCurrent(replacement);
    editor.commands.focus();
  };

  const replaceAll = () => {
    if (scope === "chapter") {
      editor.commands.replaceAllInChapter(replacement);
      editor.commands.focus();
      return;
    }
    const regex = buildRegex(query, options);
    if (!regex) return;
    let totalReplaced = 0;
    let touched = 0;
    const s = getSearchState(editor.state);
    const activeCount = s?.matches.length ?? 0;
    if (activeCount > 0) {
      editor.commands.replaceAllInChapter(replacement);
      totalReplaced += activeCount;
      touched++;
    }
    for (const c of chapters) {
      if (c.id === activeChapterId) continue;
      const result = replaceInContent(c.content, regex, replacement);
      if (result.count > 0) {
        setChapterContent(c.id, result.content);
        totalReplaced += result.count;
        touched++;
      }
    }
    setNotice(`Replaced ${totalReplaced} across ${touched} chapter${touched === 1 ? "" : "s"}`);
    editor.commands.focus();
  };

  return (
    <div className="find-bar glass">
      <button
        className="find-expand"
        data-on={expanded}
        title={expanded ? "Hide replace" : "Show replace"}
        onClick={() => setExpanded((v) => !v)}
      >
        <Icon d={expanded ? "M6 9l6 6 6-6" : "M9 6l6 6-6 6"} size={14} />
      </button>

      <div className="find-stack">
        <div className="find-row">
          <input
            ref={findInputRef}
            className="find-input"
            value={query}
            placeholder="Find"
            spellCheck={false}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onFindKey}
          />
          <span className="find-count">{countLabel}</span>
          <button className="find-btn" title="Previous (⇧⏎)" disabled={!total} onClick={prev}>
            <Icon d="M6 15l6-6 6 6" size={14} />
          </button>
          <button className="find-btn" title="Next (⏎)" disabled={!total} onClick={next}>
            <Icon d="M6 9l6 6 6-6" size={14} />
          </button>
          <button className="find-toggle" data-on={caseSensitive} title="Match case" onClick={() => setCaseSensitive((v) => !v)}>
            Aa
          </button>
          <button className="find-toggle" data-on={wholeWord} title="Whole word" onClick={() => setWholeWord((v) => !v)}>
            <span className="find-ww">ab</span>
          </button>
          <button className="find-btn" title="Close (Esc)" onClick={onClose}>
            <Icon d="M18 6L6 18M6 6l12 12" size={14} />
          </button>
        </div>

        {expanded && (
          <div className="find-row">
            <input
              className="find-input"
              value={replacement}
              placeholder="Replace"
              spellCheck={false}
              onChange={(e) => setReplacement(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  replaceOne();
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  onClose();
                }
              }}
            />
            <button className="find-action" disabled={!active.count} onClick={replaceOne} title="Replace current match">
              Replace
            </button>
            <button className="find-action" disabled={!total} onClick={replaceAll} title="Replace all matches">
              Replace All
            </button>
            <button
              className="find-scope"
              title="Replace All scope"
              onClick={() => setScope((s) => (s === "chapter" ? "book" : "chapter"))}
            >
              {scope === "chapter" ? "This chapter" : "Whole book"}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
