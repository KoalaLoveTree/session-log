package log.session

import android.content.Context
import android.os.Handler
import android.os.Looper
import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.Executors

/** Sends the phone copy to `POST /api/sync` and stores the answer. */
object Sync {
    private val worker = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())

    fun onOpen(context: Context, done: (String?) -> Unit = {}) {
        val app = context.applicationContext
        worker.execute {
            val message = runCatching { exchange(app) }.getOrElse { UNREACHABLE }
            main.post { done(message) }
        }
    }

    fun decline(context: Context, id: String, done: (String?) -> Unit) {
        val app = context.applicationContext
        worker.execute {
            val message = runCatching { declineNote(app, id) }.getOrElse { UNREACHABLE }
            main.post { done(message) }
        }
    }

    private fun exchange(context: Context): String? {
        val origin = origin(context) ?: return UNREACHABLE
        repeat(2) {
            val baseline = Copy.snapshot(context)
            val (code, text) = post("$origin/api/sync", payload(baseline))
            if (code != 200) return failure(text)
            val parsed = parse(text) ?: return "Could not sync."
            if (Copy.apply(context, baseline, parsed.entries, parsed.purges)) return null
        }
        return null
    }

    private fun declineNote(context: Context, id: String): String? {
        val note = Copy.note(context, id) ?: run {
            Copy.clearAsk(context, id)
            return null
        }
        val origin = origin(context) ?: return UNREACHABLE
        val (code, text) = post("$origin/api/purges/$id/decline", noteJson(note).toString())
        if (code == 200 || code == 404) {
            Copy.clearAsk(context, id)
            return null
        }
        return failure(text)
    }

    private fun payload(snapshot: Copy.Snapshot): String {
        val entries = JSONArray()
        for (note in snapshot.notes) entries.put(noteJson(note))
        val purges = JSONArray()
        for (purge in snapshot.purges) {
            purges.put(JSONObject().put("id", purge.id).put("purged_at", purge.purgedAt))
        }
        return JSONObject().put("entries", entries).put("purges", purges).toString()
    }

    private fun noteJson(note: Copy.Note): JSONObject {
        return JSONObject()
            .put("id", note.id)
            .put("body", note.body)
            .put("created_at", note.createdAt)
            .put("updated_at", note.updatedAt)
            .put("deleted_at", note.deletedAt ?: JSONObject.NULL)
            .put("synced_at", note.syncedAt ?: JSONObject.NULL)
            .put("other_body", note.otherBody ?: JSONObject.NULL)
    }

    private data class Response(val entries: List<Copy.Note>, val purges: List<Copy.Purge>)

    private fun parse(text: String): Response? {
        val root = runCatching { JSONObject(text) }.getOrNull() ?: return null
        val entriesJson = root.optJSONArray("entries") ?: return null
        val purgesJson = root.optJSONArray("purges") ?: return null
        val entries = ArrayList<Copy.Note>()
        for (i in 0 until entriesJson.length()) {
            val obj = entriesJson.optJSONObject(i) ?: return null
            entries.add(noteFrom(obj) ?: return null)
        }
        val purges = ArrayList<Copy.Purge>()
        for (i in 0 until purgesJson.length()) {
            val obj = purgesJson.optJSONObject(i) ?: return null
            val id = obj.optString("id")
            val at = time(obj, "purged_at")
            if (id.isEmpty() || at == null) return null
            purges.add(Copy.Purge(id, at))
        }
        return Response(entries, purges)
    }

    private fun noteFrom(obj: JSONObject): Copy.Note? {
        val id = obj.optString("id")
        val body = obj.optString("body")
        val created = time(obj, "created_at")
        val updated = time(obj, "updated_at")
        if (id.isEmpty() || body.isBlank() || created == null || updated == null) return null
        return Copy.Note(
            id = id,
            body = body,
            createdAt = created,
            updatedAt = updated,
            deletedAt = time(obj, "deleted_at"),
            syncedAt = time(obj, "synced_at"),
            otherBody = if (!obj.has("other_body") || obj.isNull("other_body")) null else obj.optString("other_body"),
        )
    }

    private fun time(obj: JSONObject, key: String): String? {
        if (!obj.has(key) || obj.isNull(key)) return null
        val value = obj.optString(key)
        return value.ifEmpty { null }
    }

    private fun failure(text: String): String {
        val message = runCatching { JSONObject(text).optString("error") }.getOrNull()
        return if (message.isNullOrBlank()) UNREACHABLE else message
    }

    private fun origin(context: Context): String? {
        val root = Store.open(context).address.trim().trimEnd('/')
        return root.ifEmpty { null }
    }

    private fun post(url: String, body: String): Pair<Int, String> {
        val conn = (URL(url).openConnection() as HttpURLConnection).apply {
            requestMethod = "POST"
            connectTimeout = 5_000
            readTimeout = 20_000
            doOutput = true
            setRequestProperty("Content-Type", "application/json")
        }
        return try {
            conn.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
            val code = conn.responseCode
            val stream = if (code >= 400) conn.errorStream else conn.inputStream
            val text = stream?.bufferedReader()?.use { it.readText() }.orEmpty()
            code to text
        } finally {
            conn.disconnect()
        }
    }

    private const val UNREACHABLE = "The PC could not be reached."
}
