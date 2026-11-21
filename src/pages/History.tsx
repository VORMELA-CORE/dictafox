import { useEffect, useState } from "react";
import {
  ClipboardCopy,
  History as HistoryIcon,
  Search,
  Trash2,
  Type,
} from "lucide-react";
import { useApp } from "../hooks/useApp";
import type { HistoryEntry } from "../types";
import {
  PROFILE_LABELS,
  REFINEMENT_LABELS,
  formatClock,
  formatDuration,
  formatRelativeTime,
  truncate,
} from "../utils/format";
import {
  Badge,
  Button,
  EmptyState,
  IconButton,
  Modal,
  Spinner,
} from "../components/ui";

export function History() {
  const {
    history,
    refreshHistory,
    copyText,
    insertText,
    deleteHistory,
    clearHistory,
    navigate,
  } = useApp();
  const [query, setQuery] = useState("");
  const [entries, setEntries] = useState<HistoryEntry[]>(history);
  const [loading, setLoading] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  useEffect(() => {
    setEntries(history);
  }, [history]);

  useEffect(() => {
    let active = true;
    setLoading(true);
    const timer = window.setTimeout(async () => {
      const found = await refreshHistory(query.trim() || undefined);
      if (active) {
        setEntries(found);
        setLoading(false);
      }
    }, 220);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [query, refreshHistory]);

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>History</h1>
          <p className="page-subtitle">
            Stored locally. Disable it or delete everything at any time.
          </p>
        </div>
        <div className="page-actions">
          <label className="search-box">
            <Search size={16} aria-hidden="true" />
            <span className="sr-only">Search transcripts</span>
            <input
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search transcripts"
            />
          </label>
          <Button
            variant="ghost"
            size="sm"
            icon={<Trash2 size={15} />}
            onClick={() => setConfirmClear(true)}
            disabled={!entries.length && !history.length}
          >
            Clear all
          </Button>
        </div>
      </header>

      <section className="card history-card">
        <div className="card-heading">
          <h2>{query ? "Search results" : "All transcripts"}</h2>
          <span className="muted">
            {loading ? <Spinner label="Searching" /> : `${entries.length} shown`}
          </span>
        </div>

        {!entries.length && !loading ? (
          <EmptyState
            icon={<HistoryIcon size={22} />}
            title={query ? "No matching transcripts" : "No transcripts yet"}
            action={
              !query ? (
                <Button variant="primary" size="sm" onClick={() => navigate("dashboard")}>
                  Open dashboard
                </Button>
              ) : (
                <Button size="sm" variant="ghost" onClick={() => setQuery("")}>
                  Clear search
                </Button>
              )
            }
          >
            {query
              ? "Try a shorter phrase. Search is case-insensitive and matches the cleaned transcript."
              : "Dictate something and it will appear here. Nothing is uploaded."}
          </EmptyState>
        ) : (
          <ul className="history-list">
            {entries.map((entry) => (
              <li className="history-row" key={entry.id}>
                <div className="history-main">
                  <p className="history-text">{entry.text}</p>
                  {entry.rawText && entry.rawText !== entry.text ? (
                    <details className="raw-details">
                      <summary>Raw Whisper output</summary>
                      <p>{truncate(entry.rawText, 500)}</p>
                    </details>
                  ) : null}
                  <div className="history-meta">
                    <span title={new Date(entry.createdAt).toLocaleString()}>
                      {formatRelativeTime(entry.createdAt)} ·{" "}
                      {formatClock(entry.createdAt)}
                    </span>
                    <Badge>{PROFILE_LABELS[entry.profile]}</Badge>
                    <Badge>{REFINEMENT_LABELS[entry.refinement]}</Badge>
                    <Badge>{entry.model}</Badge>
                    <span>{entry.language === "auto" ? "auto language" : entry.language}</span>
                    <span>{formatDuration(entry.durationMs)} audio</span>
                  </div>
                </div>
                <div className="history-actions">
                  <IconButton
                    label="Copy transcript"
                    onClick={() => void copyText(entry.text)}
                  >
                    <ClipboardCopy size={16} />
                  </IconButton>
                  <IconButton
                    label="Insert into focused app"
                    onClick={() => void insertText(entry.text)}
                  >
                    <Type size={16} />
                  </IconButton>
                  <IconButton
                    label="Delete transcript"
                    onClick={() => void deleteHistory(entry.id)}
                  >
                    <Trash2 size={16} />
                  </IconButton>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>

      {confirmClear ? (
        <Modal
          title="Delete all transcripts?"
          description="This removes every stored transcript from DictaFox's local database. It cannot be undone."
          onClose={() => setConfirmClear(false)}
          footer={
            <>
              <Button variant="ghost" onClick={() => setConfirmClear(false)}>
                Keep transcripts
              </Button>
              <Button
                variant="danger"
                onClick={async () => {
                  await clearHistory();
                  setEntries([]);
                  setConfirmClear(false);
                }}
              >
                Delete everything
              </Button>
            </>
          }
        />
      ) : null}
    </div>
  );
}
