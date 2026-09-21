import { FormEvent, KeyboardEvent, useEffect, useState } from "react";

type Entry = {
  id: number;
  body: string;
  created_at: string;
};

type Marker = "-" | "*";

type Block =
  | { kind: "p"; text: string }
  | { kind: "ul"; marker: Marker; items: string[] };

function parseBody(body: string): Block[] {
  const blocks: Block[] = [];
  for (const line of body.split("\n")) {
    const bullet = line.match(/^([*-]) (.*)$/);
    if (bullet) {
      const marker = bullet[1] as Marker;
      const item = bullet[2];
      const last = blocks[blocks.length - 1];
      if (last && last.kind === "ul" && last.marker === marker) {
        last.items.push(item);
      } else {
        blocks.push({ kind: "ul", marker, items: [item] });
      }
      continue;
    }
    const last = blocks[blocks.length - 1];
    if (last && last.kind === "p") {
      last.text += "\n" + line;
    } else {
      blocks.push({ kind: "p", text: line });
    }
  }
  return blocks;
}

function continueList(
  value: string,
  start: number,
  end: number,
): { value: string; cursor: number } | null {
  const lineStart = value.lastIndexOf("\n", start - 1) + 1;
  const lineToCaret = value.slice(lineStart, start);
  const match = lineToCaret.match(/^([*-] )/);
  if (!match) {
    return null;
  }
  const marker = match[1];
  const lineEnd = value.indexOf("\n", start);
  const fullLine = value.slice(
    lineStart,
    lineEnd === -1 ? value.length : lineEnd,
  );
  if (fullLine === marker && start === end) {
    return {
      value: value.slice(0, lineStart) + value.slice(start),
      cursor: lineStart,
    };
  }
  const insert = "\n" + marker;
  return {
    value: value.slice(0, start) + insert + value.slice(end),
    cursor: start + insert.length,
  };
}

function formatWhen(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    weekday: "short",
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  });
}

function EntryBody({ body }: { body: string }) {
  return (
    <div className="body">
      {parseBody(body).map((block, index) =>
        block.kind === "p" ? (
          <p key={index}>{block.text}</p>
        ) : (
          <ul key={index} className={block.marker === "-" ? "dash" : "star"}>
            {block.items.map((item, itemIndex) => (
              <li key={itemIndex}>{item}</li>
            ))}
          </ul>
        ),
      )}
    </div>
  );
}

export default function App() {
  const deletedPage =
    window.location.pathname.replace(/\/$/, "") === "/deleted";
  const [body, setBody] = useState("");
  const [entries, setEntries] = useState<Entry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editBody, setEditBody] = useState("");

  async function load() {
    const res = await fetch(
      deletedPage ? "/api/entries/deleted" : "/api/entries",
    );
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

  function onKeyDown(
    event: KeyboardEvent<HTMLTextAreaElement>,
    setValue: (value: string) => void,
  ) {
    if (event.key !== "Enter" || event.nativeEvent.isComposing) {
      return;
    }
    const insertingNewline =
      event.shiftKey || window.matchMedia("(pointer: coarse)").matches;
    if (!insertingNewline) {
      event.preventDefault();
      event.currentTarget.form?.requestSubmit();
      return;
    }
    const el = event.currentTarget;
    const edit = continueList(el.value, el.selectionStart, el.selectionEnd);
    if (!edit) {
      return;
    }
    event.preventDefault();
    setValue(edit.value);
    const pos = edit.cursor;
    requestAnimationFrame(() => {
      el.setSelectionRange(pos, pos);
    });
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

  async function onRestore(id: number) {
    setError(null);
    setBusy(true);
    try {
      const res = await fetch(`/api/entries/${id}/restore`, { method: "POST" });
      if (!res.ok) {
        let message = "could not restore";
        try {
          const data: { error?: string } = await res.json();
          message = data.error ?? message;
        } catch {
          /* ignore */
        }
        setError(message);
        return;
      }
      await load();
    } catch {
      setError("could not restore");
    } finally {
      setBusy(false);
    }
  }

  async function onPurge(id: number) {
    if (!confirm("Are you sure?")) {
      return;
    }
    setError(null);
    setBusy(true);
    try {
      const res = await fetch(`/api/entries/${id}/purge`, { method: "DELETE" });
      if (!res.ok) {
        let message = "could not purge";
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
      setError("could not purge");
    } finally {
      setBusy(false);
    }
  }

  function startEdit(entry: Entry) {
    setError(null);
    setEditingId(entry.id);
    setEditBody(entry.body);
  }

  async function onSaveEdit(event: FormEvent, id: number) {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const res = await fetch(`/api/entries/${id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ body: editBody }),
      });
      const data: { error?: string } = await res.json();
      if (!res.ok) {
        setError(data.error ?? "could not save");
        return;
      }
      setEditingId(null);
      await load();
    } catch {
      setError("could not save");
    } finally {
      setBusy(false);
    }
  }

  if (deletedPage) {
    return (
      <main>
        <h1>Deleted</h1>
        <p className="nav">
          <a href="/">Back</a>
        </p>
        {error ? <p className="error">{error}</p> : null}
        {entries === null ? null : entries.length === 0 ? (
          <p>No deleted entries.</p>
        ) : (
          <ul>
            {entries.map((entry) => (
              <li key={entry.id}>
                <div className="meta">
                  <time dateTime={entry.created_at}>
                    {formatWhen(entry.created_at)}
                  </time>
                  <button
                    type="button"
                    onClick={() => onRestore(entry.id)}
                    disabled={busy}
                  >
                    Restore
                  </button>
                  <button
                    type="button"
                    onClick={() => onPurge(entry.id)}
                    disabled={busy}
                  >
                    Purge
                  </button>
                </div>
                <EntryBody body={entry.body} />
              </li>
            ))}
          </ul>
        )}
      </main>
    );
  }

  return (
    <main>
      <h1>session log</h1>
      <p className="nav">
        <a href="/deleted">Deleted</a>
      </p>
      <form onSubmit={onSubmit}>
        <textarea
          value={body}
          onChange={(event) => setBody(event.target.value)}
          onKeyDown={(event) => onKeyDown(event, setBody)}
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
                  {formatWhen(entry.created_at)}
                </time>
                {editingId === entry.id ? null : (
                  <button
                    type="button"
                    onClick={() => startEdit(entry)}
                    disabled={busy}
                  >
                    Edit
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => onDelete(entry.id)}
                  disabled={busy}
                >
                  Delete
                </button>
              </div>
              {editingId === entry.id ? (
                <form onSubmit={(event) => onSaveEdit(event, entry.id)}>
                  <textarea
                    value={editBody}
                    onChange={(event) => setEditBody(event.target.value)}
                    onKeyDown={(event) => onKeyDown(event, setEditBody)}
                    rows={4}
                  />
                  <div className="actions">
                    <button type="submit" disabled={busy}>
                      Save
                    </button>
                    <button
                      type="button"
                      onClick={() => setEditingId(null)}
                      disabled={busy}
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              ) : (
                <EntryBody body={entry.body} />
              )}
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}
