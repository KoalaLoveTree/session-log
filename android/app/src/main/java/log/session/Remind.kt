package log.session

import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.os.Build
import android.os.Handler
import android.os.Looper
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.time.DayOfWeek
import java.time.Duration
import java.time.Instant
import java.time.LocalTime
import java.time.ZonedDateTime
import java.util.concurrent.Executors

class Store(private val prefs: SharedPreferences) {
    fun time(day: DayOfWeek): LocalTime? =
        prefs.getString(dayKey(day), null)?.let { runCatching { LocalTime.parse(it) }.getOrNull() }

    fun setTime(day: DayOfWeek, time: LocalTime?) = edit { putString(dayKey(day), time?.toString()) }

    var address: String
        get() = prefs.getString("address", null)?.takeIf { it.isNotBlank() } ?: ADDRESS
        set(value) = edit { putString("address", value.trim()) }

    var later: Duration?
        get() = prefs.getInt("later_minutes", 0).takeIf { it > 0 }?.let { Duration.ofMinutes(it.toLong()) }
        set(value) = edit { putInt("later_minutes", value?.toMinutes()?.toInt() ?: 0) }

    fun dues(): Set<Long> = longs("due")

    fun setDues(values: Set<Long>) = putLongs("due", values)

    fun addDue(at: Long): Boolean {
        if (at in cleared() || at in dues()) return false
        setDues(dues() + at)
        return true
    }

    var bannerDismissed: Boolean
        get() = prefs.getBoolean("banner_dismissed", false)
        set(value) = edit { putBoolean("banner_dismissed", value) }

    fun cleared(): Set<Long> = longs("cleared")

    fun setCleared(values: Set<Long>) = putLongs("cleared", values)

    fun removeClearedBy(created: List<Instant>): Set<Long> = synchronized(lock) {
        val current = longs("due")
        val removed = current.filter { Schedule.clearedBy(Instant.ofEpochMilli(it), created) }.toSet()
        if (removed.isEmpty()) return removed
        val waitsLeft = waits().filterKeys { it !in removed }
        prefs.edit()
            .putStringSet("due", (current - removed).map { it.toString() }.toSet())
            .putStringSet("waits", waitsLeft.map { "${it.key}:${it.value}" }.toSet())
            .putStringSet("cleared", (longs("cleared") + removed).map { it.toString() }.toSet())
            .commit()
        removed
    }

    fun waits(): Map<Long, Long> =
        prefs.getStringSet("waits", emptySet()).orEmpty().mapNotNull { raw ->
            val parts = raw.split(":")
            if (parts.size != 2) return@mapNotNull null
            val due = parts[0].toLongOrNull() ?: return@mapNotNull null
            val at = parts[1].toLongOrNull() ?: return@mapNotNull null
            due to at
        }.toMap()

    fun waitFor(due: Long): Long? = waits()[due]

    fun setWait(due: Long, at: Long) {
        val next = waits().toMutableMap()
        next[due] = at
        putWaits(next)
    }

    fun clearWait(due: Long) {
        val next = waits().toMutableMap()
        next.remove(due)
        putWaits(next)
    }

    fun armed(day: DayOfWeek): Long? = prefs.getLong(armedKey(day), 0L).takeIf { it > 0L }

    fun setArmed(day: DayOfWeek, at: Long?) = edit { putLong(armedKey(day), at ?: 0L) }

    private fun longs(key: String): Set<Long> =
        prefs.getStringSet(key, emptySet()).orEmpty().mapNotNull { it.toLongOrNull() }.toSet()

    private fun putLongs(key: String, values: Set<Long>) =
        edit { putStringSet(key, values.map { it.toString() }.toSet()) }

    private fun putWaits(values: Map<Long, Long>) =
        edit { putStringSet("waits", values.map { "${it.key}:${it.value}" }.toSet()) }

    private fun edit(block: SharedPreferences.Editor.() -> Unit) {
        synchronized(lock) {
            prefs.edit().apply(block).commit()
        }
    }

    companion object {
        const val ADDRESS = "http://session-log.local:3000"
        private val lock = Any()

        fun open(context: Context) =
            Store(context.applicationContext.getSharedPreferences("reminder", Context.MODE_PRIVATE))

        private fun dayKey(day: DayOfWeek) = "day_${day.value}"
        private fun armedKey(day: DayOfWeek) = "armed_${day.value}"
    }
}

object Alarms {
    fun setWeekday(context: Context, day: DayOfWeek, at: Long) {
        schedule(context, at, weekdayIntent(context, day, at))
    }

    fun cancelWeekday(context: Context, day: DayOfWeek) {
        val alarms = context.getSystemService(AlarmManager::class.java)
        alarms.cancel(weekdayIntent(context, day, 0L))
    }

    fun setWait(context: Context, due: Long, at: Long) {
        schedule(context, at, waitIntent(context, due, at))
    }

    fun cancelWait(context: Context, due: Long) {
        val alarms = context.getSystemService(AlarmManager::class.java)
        alarms.cancel(waitIntent(context, due, 0L))
    }

    fun scheduleCheck(context: Context) {
        val at = System.currentTimeMillis() + CHECK_IN
        val alarms = context.getSystemService(AlarmManager::class.java)
        alarms.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, checkIntent(context))
    }

    fun cancelCheck(context: Context) {
        val alarms = context.getSystemService(AlarmManager::class.java)
        alarms.cancel(checkIntent(context))
    }

    private fun schedule(context: Context, at: Long, pending: PendingIntent) {
        val alarms = context.getSystemService(AlarmManager::class.java)
        val exact = Build.VERSION.SDK_INT < Build.VERSION_CODES.S || alarms.canScheduleExactAlarms()
        if (exact) {
            alarms.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pending)
        } else {
            alarms.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pending)
        }
    }

    private fun weekdayIntent(context: Context, day: DayOfWeek, at: Long): PendingIntent {
        val intent = Intent(context, WeekdayReceiver::class.java).apply {
            putExtra(EXTRA_DAY, day.value)
            putExtra(EXTRA_AT, at)
        }
        return PendingIntent.getBroadcast(context, day.value, intent, FLAGS)
    }

    private fun waitIntent(context: Context, due: Long, at: Long): PendingIntent {
        val intent = Intent(context, WaitReceiver::class.java).apply {
            putExtra(EXTRA_DUE, due)
            putExtra(EXTRA_AT, at)
        }
        return PendingIntent.getBroadcast(context, waitCode(due), intent, FLAGS)
    }

    private fun checkIntent(context: Context): PendingIntent =
        PendingIntent.getBroadcast(context, CHECK_CODE, Intent(context, CheckReceiver::class.java), FLAGS)

    private fun waitCode(due: Long): Int {
        val mixed = (due xor (due ushr 32)).toInt() and 0x7fffffff
        return if (mixed < 8) mixed + 8 else mixed
    }

    private const val FLAGS = PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
    private const val CHECK_CODE = 9
    private const val CHECK_IN = 60_000L
}

object Banner {
    private const val CHANNEL = "reminder"
    private const val ID = 1

    fun sync(context: Context, post: Boolean) {
        val nm = context.getSystemService(NotificationManager::class.java)
        val store = Store.open(context)
        val now = System.currentTimeMillis()
        val showing = store.dues().any { due ->
            val wait = store.waitFor(due)
            wait == null || wait <= now
        }
        if (store.dues().isEmpty()) {
            nm.cancel(ID)
            store.bannerDismissed = false
            Alarms.cancelCheck(context)
            return
        }
        Alarms.scheduleCheck(context)
        if (!showing) {
            nm.cancel(ID)
            return
        }
        val canPost = Build.VERSION.SDK_INT < 33 ||
            context.checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) ==
            android.content.pm.PackageManager.PERMISSION_GRANTED
        if (!post || store.bannerDismissed || !canPost) return
        if (nm.activeNotifications.any { it.id == ID }) return
        val channel = NotificationChannel(CHANNEL, "Reminder", NotificationManager.IMPORTANCE_HIGH)
        nm.createNotificationChannel(channel)
        val open = PendingIntent.getActivity(
            context,
            0,
            Intent(Intent.ACTION_VIEW, android.net.Uri.parse(store.address)).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val dismiss = PendingIntent.getBroadcast(
            context,
            1,
            Intent(context, DismissReceiver::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val note = Notification.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_banner)
            .setContentTitle("Write a line")
            .setContentIntent(open)
            .setDeleteIntent(dismiss)
            .setAutoCancel(false)
            .setOnlyAlertOnce(true)
            .build()
        nm.notify(ID, note)
    }
}

object LogCheck {
    fun createdAt(origin: String): List<Instant>? {
        val root = origin.trim().trimEnd('/')
        if (root.isEmpty()) return null
        val conn = (URL("$root/api/entries").openConnection() as HttpURLConnection).apply {
            connectTimeout = 5000
            readTimeout = 5000
            instanceFollowRedirects = true
        }
        return try {
            if (conn.responseCode != 200) return null
            val text = conn.inputStream.bufferedReader().use { it.readText() }
            val entries = JSONObject(text).optJSONArray("entries") ?: return null
            buildList {
                for (i in 0 until entries.length()) {
                    val raw = entries.optJSONObject(i)?.optString("created_at").orEmpty()
                    if (raw.isEmpty()) continue
                    val at = runCatching { Instant.parse(raw) }.getOrNull() ?: continue
                    add(at)
                }
            }
        } catch (_: Exception) {
            null
        } finally {
            conn.disconnect()
        }
    }
}

object Remind {
    private val io = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())

    fun open(context: Context, done: (String?) -> Unit) {
        catchUp(context, ZonedDateTime.now())
        refresh(context, post = true, done)
    }

    fun setDay(context: Context, day: DayOfWeek, time: LocalTime?) {
        val store = Store.open(context)
        if (time == null) {
            store.setTime(day, null)
            store.setArmed(day, null)
            Alarms.cancelWeekday(context, day)
            return
        }
        store.setTime(day, time)
        arm(context, store, day, Schedule.nextWeekly(day, time, ZonedDateTime.now()))
    }

    fun later(context: Context, due: Long, duration: Duration) {
        val at = Schedule.laterWait(duration, ZonedDateTime.now()).toInstant().toEpochMilli()
        wait(context, due, at)
    }

    fun whenAt(context: Context, due: Long, clock: LocalTime) {
        val at = Schedule.whenWait(clock, ZonedDateTime.now()).toInstant().toEpochMilli()
        wait(context, due, at)
    }

    fun onWeekday(context: Context, day: DayOfWeek, scheduledAt: Long, done: () -> Unit) {
        val store = Store.open(context)
        val time = store.time(day)
        if (store.armed(day) != scheduledAt || time == null) {
            done()
            return
        }
        val now = ZonedDateTime.now()
        val past = Schedule.latestPast(day, time, now).toInstant().toEpochMilli()
        if (store.addDue(past)) store.bannerDismissed = false
        arm(context, store, day, Schedule.nextWeekly(day, time, now))
        refresh(context, post = true) { done() }
    }

    fun onWait(context: Context, due: Long, scheduledAt: Long, done: () -> Unit) {
        val store = Store.open(context)
        if (store.waitFor(due) != scheduledAt) {
            done()
            return
        }
        store.clearWait(due)
        Alarms.cancelWait(context, due)
        store.bannerDismissed = false
        refresh(context, post = true) { done() }
    }

    private fun catchUp(context: Context, now: ZonedDateTime) {
        val store = Store.open(context)
        pruneCleared(store, now)
        val nowMillis = now.toInstant().toEpochMilli()
        for (day in DayOfWeek.entries) {
            val time = store.time(day)
            if (time == null) {
                store.setArmed(day, null)
                Alarms.cancelWeekday(context, day)
                continue
            }
            val armed = store.armed(day)
            if (armed == null) {
                arm(context, store, day, Schedule.nextWeekly(day, time, now))
            } else if (armed <= nowMillis) {
                val past = Schedule.latestPast(day, time, now).toInstant().toEpochMilli()
                if (store.addDue(past)) store.bannerDismissed = false
                arm(context, store, day, Schedule.nextWeekly(day, time, now))
            } else {
                Alarms.setWeekday(context, day, armed)
            }
        }
    }

    private fun arm(context: Context, store: Store, day: DayOfWeek, at: ZonedDateTime) {
        val millis = at.toInstant().toEpochMilli()
        store.setArmed(day, millis)
        Alarms.setWeekday(context, day, millis)
    }

    private fun wait(context: Context, due: Long, at: Long) {
        val store = Store.open(context)
        if (due !in store.dues()) return
        store.setWait(due, at)
        Alarms.setWait(context, due, at)
        Banner.sync(context, post = false)
    }

    fun onCheck(context: Context, done: () -> Unit) {
        refresh(context, post = false) { done() }
    }

    private fun refresh(context: Context, post: Boolean, done: (String?) -> Unit) {
        val app = context.applicationContext
        io.execute {
            var status: String? = null
            try {
                status = pull(app, post)
            } finally {
                main.post { done(status) }
            }
        }
    }

    private fun pull(context: Context, post: Boolean): String? {
        val store = Store.open(context)
        val created = LogCheck.createdAt(store.address)
        if (created == null) {
            Banner.sync(context, post)
            return "The PC could not be reached."
        }
        val removed = store.removeClearedBy(created)
        for (due in removed) Alarms.cancelWait(context, due)
        pruneCleared(store, ZonedDateTime.now())
        Banner.sync(context, post)
        return null
    }

    private fun pruneCleared(store: Store, now: ZonedDateTime) {
        val keep = DayOfWeek.entries.mapNotNull { day ->
            val time = store.time(day) ?: return@mapNotNull null
            Schedule.latestPast(day, time, now).toInstant().toEpochMilli()
        }.toSet()
        store.setCleared(store.cleared().intersect(keep))
    }
}

const val EXTRA_DAY = "day"
const val EXTRA_DUE = "due"
const val EXTRA_AT = "at"

class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        val pending = goAsync()
        Remind.open(context) { pending.finish() }
    }
}

class WeekdayReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val day = DayOfWeek.of(intent.getIntExtra(EXTRA_DAY, DayOfWeek.MONDAY.value))
        val at = intent.getLongExtra(EXTRA_AT, 0L)
        val pending = goAsync()
        Remind.onWeekday(context, day, at) { pending.finish() }
    }
}

class WaitReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val due = intent.getLongExtra(EXTRA_DUE, 0L)
        val at = intent.getLongExtra(EXTRA_AT, 0L)
        val pending = goAsync()
        Remind.onWait(context, due, at) { pending.finish() }
    }
}

class CheckReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val pending = goAsync()
        Remind.onCheck(context) { pending.finish() }
    }
}

class DismissReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        Store.open(context).bannerDismissed = true
    }
}
