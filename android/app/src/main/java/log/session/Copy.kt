package log.session

import android.content.ContentValues
import android.content.Context
import android.database.Cursor
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import java.time.Instant
import java.time.temporal.ChronoUnit
import java.time.format.DateTimeFormatter
import java.util.UUID

/** The phone's notes. SQLite only. Writes land here before any sync. */
object Copy {
    data class Note(
        val id: String,
        val body: String,
        val createdAt: String,
        val updatedAt: String,
        val deletedAt: String?,
        val syncedAt: String?,
        val otherBody: String?,
    )

    data class Purge(val id: String, val purgedAt: String)

    data class Snapshot(val notes: List<Note>, val purges: List<Purge>, val asks: List<Purge>)

    private val lock = Any()
    private var helper: Helper? = null

    fun notes(context: Context, deleted: Boolean): List<Note> = use(context) { db ->
        val where = if (deleted) "deleted_at IS NOT NULL" else "deleted_at IS NULL"
        queryNotes(db, where, "created_at DESC")
    }

    fun asks(context: Context): List<Purge> = use(context) { db -> readAsks(db) }

    fun snapshot(context: Context): Snapshot = use(context) { db -> readSnapshot(db) }

    fun add(context: Context, body: String): Boolean = use(context) { db ->
        val trimmed = body.trim()
        if (trimmed.isEmpty()) return@use false
        val now = stamp(null)
        insert(
            db,
            Note(
                id = UUID.randomUUID().toString(),
                body = trimmed,
                createdAt = now,
                updatedAt = now,
                deletedAt = null,
                syncedAt = null,
                otherBody = null,
            ),
        )
        true
    }

    fun save(context: Context, id: String, body: String): Boolean = use(context) { db ->
        val trimmed = body.trim()
        val note = find(db, id) ?: return@use false
        if (trimmed.isEmpty()) return@use false
        val values = ContentValues().apply {
            put("body", trimmed)
            put("updated_at", stamp(note.syncedAt))
            if (note.otherBody != null) {
                putNull("other_body")
                putNull("deleted_at")
            }
        }
        db.update("entries", values, "id = ?", arrayOf(id))
        true
    }

    fun softDelete(context: Context, id: String) = use(context) { db ->
        val note = find(db, id) ?: return@use
        val now = stamp(note.syncedAt)
        val values = ContentValues().apply {
            put("deleted_at", now)
            put("updated_at", now)
        }
        db.update("entries", values, "id = ?", arrayOf(id))
    }

    fun restore(context: Context, id: String) = use(context) { db ->
        val note = find(db, id) ?: return@use
        val values = ContentValues().apply {
            putNull("deleted_at")
            put("updated_at", stamp(note.syncedAt))
        }
        db.update("entries", values, "id = ?", arrayOf(id))
    }

    /** A note that never synced is removed with no purge. A synced note keeps a purge. */
    fun purge(context: Context, id: String) = use(context) { db ->
        val note = find(db, id) ?: return@use
        if (note.syncedAt != null) rememberPurge(db, id, format(Instant.now()))
        db.delete("entries", "id = ?", arrayOf(id))
        db.delete("asks", "id = ?", arrayOf(id))
    }

    fun accept(context: Context, id: String) = use(context) { db ->
        val ask = readAsks(db).firstOrNull { it.id == id } ?: return@use
        rememberPurge(db, id, ask.purgedAt)
        db.delete("entries", "id = ?", arrayOf(id))
        db.delete("asks", "id = ?", arrayOf(id))
    }

    fun note(context: Context, id: String): Note? = use(context) { db -> find(db, id) }

    fun clearAsk(context: Context, id: String) = use(context) { db ->
        db.delete("asks", "id = ?", arrayOf(id))
    }

    /**
     * Store a sync response only when the copy is still the snapshot that was sent.
     * A local write during the request keeps the copy, and the next open sends it.
     */
    fun apply(context: Context, baseline: Snapshot, entries: List<Note>, incoming: List<Purge>): Boolean =
        use(context) { db ->
            if (readSnapshot(db) != baseline) return@use false
            val written = entries.map { it.id }.toSet()
            db.beginTransaction()
            try {
                for (note in entries) writeReturned(db, note)
                for (purge in incoming) {
                    if (purge.id in written) continue
                    if (find(db, purge.id) == null) continue
                    rememberAsk(db, purge.id, purge.purgedAt)
                }
                db.setTransactionSuccessful()
            } finally {
                db.endTransaction()
            }
            true
        }

    private fun <T> use(context: Context, block: (SQLiteDatabase) -> T): T = synchronized(lock) {
        block(database(context))
    }

    private fun database(context: Context): SQLiteDatabase {
        val open = helper ?: Helper(context.applicationContext).also { helper = it }
        return open.writableDatabase
    }

    private fun readSnapshot(db: SQLiteDatabase): Snapshot {
        return Snapshot(
            notes = queryNotes(db, null, "id"),
            purges = queryPurges(db, "purges"),
            asks = queryPurges(db, "asks"),
        )
    }

    private fun queryNotes(db: SQLiteDatabase, where: String?, order: String): List<Note> {
        db.query("entries", null, where, null, null, null, order).use { cursor ->
            val notes = ArrayList<Note>()
            while (cursor.moveToNext()) notes.add(readNote(cursor))
            return notes
        }
    }

    private fun queryPurges(db: SQLiteDatabase, table: String): List<Purge> {
        db.query(table, arrayOf("id", "purged_at"), null, null, null, null, "id").use { cursor ->
            val rows = ArrayList<Purge>()
            while (cursor.moveToNext()) {
                rows.add(Purge(cursor.getString(0), cursor.getString(1)))
            }
            return rows
        }
    }

    private fun readAsks(db: SQLiteDatabase): List<Purge> = queryPurges(db, "asks")

    private fun find(db: SQLiteDatabase, id: String): Note? {
        db.query("entries", null, "id = ?", arrayOf(id), null, null, null).use { cursor ->
            if (!cursor.moveToFirst()) return null
            return readNote(cursor)
        }
    }

    private fun readNote(cursor: Cursor): Note {
        return Note(
            id = cursor.text("id"),
            body = cursor.text("body"),
            createdAt = cursor.text("created_at"),
            updatedAt = cursor.text("updated_at"),
            deletedAt = cursor.textOrNull("deleted_at"),
            syncedAt = cursor.textOrNull("synced_at"),
            otherBody = cursor.textOrNull("other_body"),
        )
    }

    private fun insert(db: SQLiteDatabase, note: Note) {
        db.insertWithOnConflict("entries", null, values(note), SQLiteDatabase.CONFLICT_REPLACE)
    }

    private fun writeReturned(db: SQLiteDatabase, note: Note) {
        db.delete("purges", "id = ?", arrayOf(note.id))
        db.delete("asks", "id = ?", arrayOf(note.id))
        insert(db, note)
    }

    private fun rememberPurge(db: SQLiteDatabase, id: String, purgedAt: String) {
        val values = ContentValues().apply {
            put("id", id)
            put("purged_at", purgedAt)
        }
        db.insertWithOnConflict("purges", null, values, SQLiteDatabase.CONFLICT_REPLACE)
    }

    private fun rememberAsk(db: SQLiteDatabase, id: String, purgedAt: String) {
        val values = ContentValues().apply {
            put("id", id)
            put("purged_at", purgedAt)
        }
        db.insertWithOnConflict("asks", null, values, SQLiteDatabase.CONFLICT_REPLACE)
    }

    private fun values(note: Note): ContentValues {
        return ContentValues().apply {
            put("id", note.id)
            put("body", note.body)
            put("created_at", note.createdAt)
            put("updated_at", note.updatedAt)
            if (note.deletedAt == null) putNull("deleted_at") else put("deleted_at", note.deletedAt)
            if (note.syncedAt == null) putNull("synced_at") else put("synced_at", note.syncedAt)
            if (note.otherBody == null) putNull("other_body") else put("other_body", note.otherBody)
        }
    }

    private fun stamp(syncedAt: String?): String {
        val now = Instant.now().truncatedTo(ChronoUnit.MICROS)
        val synced = syncedAt?.let { runCatching { Instant.parse(it) }.getOrNull() }
        if (synced == null || now.isAfter(synced)) return format(now)
        return format(synced.plusMillis(1).truncatedTo(ChronoUnit.MICROS))
    }

    private fun format(instant: Instant): String = DateTimeFormatter.ISO_INSTANT.format(instant)

    private fun Cursor.text(name: String): String = getString(getColumnIndexOrThrow(name))

    private fun Cursor.textOrNull(name: String): String? {
        val index = getColumnIndexOrThrow(name)
        return if (isNull(index)) null else getString(index)
    }

    private class Helper(context: Context) : SQLiteOpenHelper(context, "copy.db", null, 1) {
        override fun onCreate(db: SQLiteDatabase) {
            db.execSQL(
                """
                CREATE TABLE entries (
                    id TEXT PRIMARY KEY,
                    body TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT,
                    synced_at TEXT,
                    other_body TEXT
                )
                """.trimIndent(),
            )
            db.execSQL("CREATE TABLE purges (id TEXT PRIMARY KEY, purged_at TEXT NOT NULL)")
            // Incoming PC purge waiting for Remove or Keep. Not sent on sync.
            db.execSQL("CREATE TABLE asks (id TEXT PRIMARY KEY, purged_at TEXT NOT NULL)")
        }

        override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) = Unit
    }
}
