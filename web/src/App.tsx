import { FormEvent, KeyboardEvent, useEffect, useState } from "react";

type Entry = {
  id: string;
  body: string;
  created_at: string;
  other_body: string | null;
};

type PendingPurge = {
  id: string;
  changed: boolean;
};

function newId(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

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
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
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

function purgeQuestion(changed: boolean): string {
  return changed
    ? "The phone purged this, and this copy changed after the last sync. Remove it here too?"
    : "The phone purged this. Remove it here too?";
}

function PurgeAsk({
  changed,
  body,
  busy,
  showBody,
  onRemove,
  onKeep,
}: {
  changed: boolean;
  body: string;
  busy: boolean;
  showBody: boolean;
  onRemove: () => void;
  onKeep: () => void;
}) {
  return (
    <>
      <p className="ask">{purgeQuestion(changed)}</p>
      {showBody ? (
        <div className="asked">
          <EntryBody body={body} />
        </div>
      ) : null}
      <div className="actions">
        <button type="button" onClick={onRemove} disabled={busy}>
          Remove
        </button>
        <button type="button" onClick={onKeep} disabled={busy}>
          Keep
        </button>
      </div>
    </>
  );
}

function MergeFields({
  body,
  otherBody,
  busy,
  onSave,
}: {
  body: string;
  otherBody: string;
  busy: boolean;
  onSave: (body: string) => void;
}) {
  const [remaining, setRemaining] = useState(body);
  return (
    <>
      <EntryBody body={body} />
      <div className="asked">
        <EntryBody body={otherBody} />
      </div>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          onSave(remaining);
        }}
      >
        <textarea
          value={remaining}
          onChange={(event) => setRemaining(event.target.value)}
          onKeyDown={(event) => onKeyDown(event, setRemaining)}
          rows={4}
        />
        <div className="actions">
          <button type="submit" disabled={busy}>
            Save
          </button>
        </div>
      </form>
    </>
  );
}

function otherText(entry: Entry): string | null {
  return typeof entry.other_body === "string" ? entry.other_body : null;
}

function Conflict({
  entry,
  pendingChanged,
  busy,
  onMergeSave,
  onRemove,
  onKeep,
}: {
  entry: Entry;
  pendingChanged: boolean | undefined;
  busy: boolean;
  onMergeSave: (body: string) => void;
  onRemove: () => void;
  onKeep: () => void;
}) {
  const other = otherText(entry);
  return (
    <>
      {pendingChanged !== undefined ? (
        <PurgeAsk
          changed={pendingChanged}
          body={entry.body}
          busy={busy}
          showBody={other === null}
          onRemove={onRemove}
          onKeep={onKeep}
        />
      ) : null}
      {other !== null ? (
        <MergeFields
          key={`${entry.body}\0${other}`}
          body={entry.body}
          otherBody={other}
          busy={busy}
          onSave={onMergeSave}
        />
      ) : null}
    </>
  );
}

export default function App() {
  const deletedPage =
    window.location.pathname.replace(/\/$/, "") === "/deleted";
  const [body, setBody] = useState("");
  const [entries, setEntries] = useState<Entry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editBody, setEditBody] = useState("");
  const [pending, setPending] = useState<Map<string, boolean>>(new Map());

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
    const pendingRes = await fetch("/api/purges/pending");
    if (!pendingRes.ok) {
      setError("could not load entries");
      return;
    }
    const pendingData: { purges: PendingPurge[] } = await pendingRes.json();
    const shown = new Set(data.entries.map((entry) => entry.id));
    const next = new Map<string, boolean>();
    for (const purge of pendingData.purges) {
      if (shown.has(purge.id)) {
        next.set(purge.id, purge.changed);
      }
    }
    setPending(next);
  }

  async function answerPurge(id: string, accept: boolean) {
    setError(null);
    setBusy(true);
    try {
      const res = await fetch(
        `/api/purges/${id}/${accept ? "accept" : "decline"}`,
        { method: "POST" },
      );
      if (!res.ok) {
        let detail = accept ? "could not delete" : "could not save";
        try {
          const data: { error?: string } = await res.json();
          detail = data.error ?? detail;
        } catch {
          /* a failure may have no JSON body */
        }
        setError(detail);
        return;
      }
      await load();
    } catch {
      setError(accept ? "could not delete" : "could not save");
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    load().catch(() => setError("could not load entries"));
  }, []);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const res = await fetch("/api/entries", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: newId(), body }),
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

  async function onDelete(id: string) {
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

  async function onRestore(id: string) {
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

  async function onPurge(id: string) {
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

  async function saveBody(id: string, nextBody: string) {
    setError(null);
    setBusy(true);
    try {
      const res = await fetch(`/api/entries/${id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ body: nextBody }),
      });
      const data: { error?: string } = await res.json();
      if (!res.ok) {
        setError(data.error ?? "could not save");
        return;
      }
      if (editingId === id) {
        setEditingId(null);
      }
      await load();
    } catch {
      setError("could not save");
    } finally {
      setBusy(false);
    }
  }

  function onSaveEdit(event: FormEvent, id: string) {
    event.preventDefault();
    void saveBody(id, editBody);
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
            {entries.map((entry) => {
              const changed = pending.get(entry.id);
              return (
                <li key={entry.id}>
                  <div className="meta">
                    <time dateTime={entry.created_at}>
                      {formatWhen(entry.created_at)}
                    </time>
                    {changed === undefined ? (
                      <>
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
                      </>
                    ) : null}
                  </div>
                  {changed !== undefined || otherText(entry) !== null ? (
                    <Conflict
                      entry={entry}
                      pendingChanged={changed}
                      busy={busy}
                      onMergeSave={(next) => void saveBody(entry.id, next)}
                      onRemove={() => answerPurge(entry.id, true)}
                      onKeep={() => answerPurge(entry.id, false)}
                    />
                  ) : (
                    <EntryBody body={entry.body} />
                  )}
                </li>
              );
            })}
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
          {entries.map((entry) => {
            const changed = pending.get(entry.id);
            return (
              <li key={entry.id}>
                <div className="meta">
                  <time dateTime={entry.created_at}>
                    {formatWhen(entry.created_at)}
                  </time>
                  {changed !== undefined ||
                  editingId === entry.id ||
                  otherText(entry) !== null ? null : (
                    <button
                      type="button"
                      onClick={() => startEdit(entry)}
                      disabled={busy}
                    >
                      Edit
                    </button>
                  )}
                  {changed === undefined ? (
                    <button
                      type="button"
                      onClick={() => onDelete(entry.id)}
                      disabled={busy}
                    >
                      Delete
                    </button>
                  ) : null}
                </div>
                {changed !== undefined || otherText(entry) !== null ? (
                  <Conflict
                    entry={entry}
                    pendingChanged={changed}
                    busy={busy}
                    onMergeSave={(next) => void saveBody(entry.id, next)}
                    onRemove={() => answerPurge(entry.id, true)}
                    onKeep={() => answerPurge(entry.id, false)}
                  />
                ) : editingId === entry.id ? (
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
            );
          })}
        </ul>
      )}
    </main>
  );
}
