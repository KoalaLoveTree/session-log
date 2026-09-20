import { FormEvent, KeyboardEvent, useEffect, useState } from "react";

type Entry = {
  id: number;
  body: string;
  created_at: string;
};

export default function App() {
  const [body, setBody] = useState("");
  const [entries, setEntries] = useState<Entry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function load() {
    const res = await fetch("/api/entries");
    if (!res.ok) {
      setError("could not load entries");
      return;
    }
    const data: { entries: Entry[] } = await res.json();
    setEntries(data.entries);
  }

  useEffect(() => {
    load().catch(() => setError("could not load entries"));
  }, []);

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) {
      return;
    }
    event.preventDefault();
    event.currentTarget.form?.requestSubmit();
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const res = await fetch("/api/entries", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ body }),
      });
      const data: { error?: string } = await res.json();
      if (!res.ok) {
        setError(data.error ?? "could not save");
        return;
      }
      setBody("");
      await load();
    } catch {
      setError("could not save");
    } finally {
      setBusy(false);
    }
  }

  async function onDelete(id: number) {
    if (!confirm("Are you sure?")) {
      return;
    }
    setError(null);
    setBusy(true);
    try {
      const res = await fetch(`/api/entries/${id}`, { method: "DELETE" });
      if (!res.ok) {
        let message = "could not delete";
        try {
          const data: { error?: string } = await res.json();
          message = data.error ?? message;
        } catch {
          /* 204 has no body */
        }
        setError(message);
        return;
      }
      await load();
    } catch {
      setError("could not delete");
    } finally {
      setBusy(false);
    }
  }

  return (
    <main>
      <h1>lr</h1>
      <form onSubmit={onSubmit}>
        <textarea
          value={body}
          onChange={(event) => setBody(event.target.value)}
          onKeyDown={onKeyDown}
          rows={4}
        />
        <button type="submit" disabled={busy}>
          Add
        </button>
      </form>
      {error ? <p className="error">{error}</p> : null}
      {entries === null ? null : entries.length === 0 ? (
        <p>No entries yet.</p>
      ) : (
        <ul>
          {entries.map((entry) => (
            <li key={entry.id}>
              <div className="meta">
                <time dateTime={entry.created_at}>
                  {new Date(entry.created_at).toLocaleString()}
                </time>
                <button
                  type="button"
                  onClick={() => onDelete(entry.id)}
                  disabled={busy}
                >
                  Delete
                </button>
              </div>
              <p>{entry.body}</p>
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}
