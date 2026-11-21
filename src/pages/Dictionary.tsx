import { useMemo, useState } from "react";
import {
  BookOpen,
  Check,
  Pencil,
  Plus,
  Search,
  Trash2,
  X,
} from "lucide-react";
import { useApp } from "../hooks/useApp";
import type { DictionaryInput } from "../types";
import {
  Badge,
  Button,
  EmptyState,
  Field,
  IconButton,
  Toggle,
} from "../components/ui";

interface FormState {
  id: number | null;
  spoken: string;
  written: string;
  category: string;
  caseSensitive: boolean;
  enabled: boolean;
}

const EMPTY_FORM: FormState = {
  id: null,
  spoken: "",
  written: "",
  category: "",
  caseSensitive: false,
  enabled: true,
};

export function Dictionary() {
  const { dictionary, upsertDictionary, deleteDictionary, settings, saveSettings } =
    useApp();
  const [form, setForm] = useState<FormState>(EMPTY_FORM);
  const [formError, setFormError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [saving, setSaving] = useState(false);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return dictionary;
    return dictionary.filter((entry) =>
      [entry.spoken, entry.written, entry.category ?? ""]
        .join(" ")
        .toLowerCase()
        .includes(needle),
    );
  }, [dictionary, query]);

  const save = async () => {
    setFormError(null);
    if (!form.spoken.trim() || !form.written.trim()) {
      setFormError("Both the spoken and written forms are required.");
      return;
    }
    setSaving(true);
    const input: DictionaryInput = {
      id: form.id,
      spoken: form.spoken,
      written: form.written,
      category: form.category.trim() || null,
      caseSensitive: form.caseSensitive,
      enabled: form.enabled,
    };
    const saved = await upsertDictionary(input);
    setSaving(false);
    if (saved) {
      setForm(EMPTY_FORM);
    } else {
      setFormError("The entry could not be saved. Check the values and try again.");
    }
  };

  const startEdit = (id: number) => {
    const entry = dictionary.find((item) => item.id === id);
    if (!entry) return;
    setForm({
      id: entry.id,
      spoken: entry.spoken,
      written: entry.written,
      category: entry.category ?? "",
      caseSensitive: entry.caseSensitive,
      enabled: entry.enabled,
    });
    setFormError(null);
  };

  const toggleEntry = async (id: number, enabled: boolean) => {
    const entry = dictionary.find((item) => item.id === id);
    if (!entry) return;
    await upsertDictionary({
      id: entry.id,
      spoken: entry.spoken,
      written: entry.written,
      category: entry.category,
      caseSensitive: entry.caseSensitive,
      enabled,
    });
  };

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>Personal dictionary</h1>
          <p className="page-subtitle">
            Teach DictaFox names, jargon and spellings. Applied locally before
            the transcript reaches any app.
          </p>
        </div>
        <div className="page-actions">
          <label className="search-box">
            <Search size={16} aria-hidden="true" />
            <span className="sr-only">Search dictionary</span>
            <input
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search words"
            />
          </label>
        </div>
      </header>

      <div className="dictionary-layout">
        <section className="card dictionary-form">
          <div className="card-heading">
            <h2>{form.id ? "Edit entry" : "Add entry"}</h2>
            {form.id ? (
              <IconButton label="Cancel editing" onClick={() => setForm(EMPTY_FORM)}>
                <X size={16} />
              </IconButton>
            ) : (
              <Badge tone="accent">
                <Plus size={12} aria-hidden="true" /> New
              </Badge>
            )}
          </div>
          <Field
            label="Spoken form"
            hint="How Whisper usually hears it, e.g. “doc type”."
          >
            <input
              type="text"
              value={form.spoken}
              maxLength={200}
              onChange={(event) =>
                setForm({ ...form, spoken: event.target.value })
              }
              placeholder="doc type"
            />
          </Field>
          <Field
            label="Written form"
            hint="How it should appear, e.g. “DocType”."
          >
            <input
              type="text"
              value={form.written}
              maxLength={200}
              onChange={(event) =>
                setForm({ ...form, written: event.target.value })
              }
              placeholder="DocType"
            />
          </Field>
          <Field label="Category (optional)">
            <input
              type="text"
              value={form.category}
              maxLength={60}
              onChange={(event) =>
                setForm({ ...form, category: event.target.value })
              }
              placeholder="Frappe, tools, people…"
            />
          </Field>
          <Toggle
            checked={form.caseSensitive}
            onChange={(checked) => setForm({ ...form, caseSensitive: checked })}
            label="Case-sensitive match"
            description="Only replace when the spoken form has this exact casing."
          />
          <Toggle
            checked={form.enabled}
            onChange={(checked) => setForm({ ...form, enabled: checked })}
            label="Enabled"
            description="Disabled entries are kept but ignored."
          />
          {formError ? <p className="form-error">{formError}</p> : null}
          <Button
            variant="primary"
            onClick={() => void save()}
            disabled={saving}
            icon={<Check size={15} />}
          >
            {saving ? "Saving…" : form.id ? "Save changes" : "Add to dictionary"}
          </Button>
        </section>

        <section className="card dictionary-list-card">
          <div className="card-heading">
            <h2>Entries</h2>
            <div className="dictionary-tools">
              <Badge>{dictionary.length} total</Badge>
              <Toggle
                checked={settings?.dictionaryEnabled ?? true}
                onChange={(checked) =>
                  settings &&
                  void saveSettings({ ...settings, dictionaryEnabled: checked })
                }
                label="Use dictionary"
              />
            </div>
          </div>
          {filtered.length ? (
            <ul className="dictionary-list">
              {filtered.map((entry) => (
                <li key={entry.id} className="dictionary-row">
                  <div className="dictionary-words">
                    <span className="spoken">{entry.spoken}</span>
                    <span className="arrow" aria-hidden="true">
                      →
                    </span>
                    <span className="written">{entry.written}</span>
                    {entry.category ? <Badge>{entry.category}</Badge> : null}
                    {entry.caseSensitive ? <Badge tone="warning">Case</Badge> : null}
                    {!entry.enabled ? <Badge tone="danger">Off</Badge> : null}
                  </div>
                  <div className="dictionary-actions">
                    <Toggle
                      checked={entry.enabled}
                      onChange={(checked) => void toggleEntry(entry.id, checked)}
                      label={`${entry.written} enabled`}
                    />
                    <IconButton
                      label={`Edit ${entry.written}`}
                      onClick={() => startEdit(entry.id)}
                    >
                      <Pencil size={15} />
                    </IconButton>
                    <IconButton
                      label={`Delete ${entry.written}`}
                      onClick={() => void deleteDictionary(entry.id)}
                    >
                      <Trash2 size={15} />
                    </IconButton>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyState
              icon={<BookOpen size={22} />}
              title={query ? "No matching entries" : "Your dictionary is empty"}
              action={
                query ? (
                  <Button size="sm" variant="ghost" onClick={() => setQuery("")}>
                    Clear filter
                  </Button>
                ) : (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() =>
                      setForm({
                        ...EMPTY_FORM,
                        spoken: "doc type",
                        written: "DocType",
                        category: "Frappe",
                      })
                    }
                  >
                    Try an example
                  </Button>
                )
              }
            >
              {query
                ? "Try a different word or category."
                : "Add technical terms, product names and people so they are spelled your way."}
            </EmptyState>
          )}
        </section>
      </div>
    </div>
  );
}
