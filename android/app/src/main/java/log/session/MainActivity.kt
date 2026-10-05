package log.session

import android.Manifest
import android.app.AlarmManager
import android.app.TimePickerDialog
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.NumberPicker
import android.widget.Switch
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import java.time.DayOfWeek
import java.time.Duration
import java.time.Instant
import java.time.LocalTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.TextStyle
import java.util.Locale

class MainActivity : AppCompatActivity() {
    private lateinit var address: EditText
    private lateinit var later: Button
    private lateinit var status: TextView
    private lateinit var exact: TextView
    private lateinit var allowExact: Button
    private lateinit var days: LinearLayout
    private lateinit var dueList: LinearLayout

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        address = findViewById(R.id.address)
        later = findViewById(R.id.later)
        status = findViewById(R.id.status)
        exact = findViewById(R.id.exact)
        allowExact = findViewById(R.id.allow_exact)
        days = findViewById(R.id.days)
        dueList = findViewById(R.id.due_list)

        val store = Store.open(this)
        address.setText(store.address)
        showDuration(store.later)
        later.setOnClickListener {
            pickDuration(Store.open(this).later) { duration ->
                Store.open(this).later = duration
                showDuration(duration)
            }
        }
        allowExact.setOnClickListener {
            startActivity(
                Intent(Settings.ACTION_REQUEST_SCHEDULE_EXACT_ALARM).apply {
                    data = Uri.parse("package:$packageName")
                },
            )
        }
        if (Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 1)
        }
        for (day in DayOfWeek.entries) addDay(day)
    }

    override fun onResume() {
        super.onResume()
        showExact()
        renderDues()
        Remind.open(this) { message ->
            status.text = message.orEmpty()
            renderDues()
        }
    }

    override fun onPause() {
        saveAddress()
        super.onPause()
    }

    private fun addDay(day: DayOfWeek) {
        val store = Store.open(this)
        val stored = store.time(day)
        val row = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        val label = TextView(this).apply {
            text = day.getDisplayName(TextStyle.FULL, Locale.getDefault())
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }
        val time = Button(this).apply {
            text = stored?.format(CLOCK) ?: "Off"
            isEnabled = stored != null
        }
        val toggle = Switch(this).apply { isChecked = stored != null }
        time.setOnClickListener {
            pickTime(store.time(day)) { chosen ->
                if (chosen == null) return@pickTime
                Remind.setDay(this, day, chosen)
                time.text = chosen.format(CLOCK)
                time.isEnabled = true
                toggle.isChecked = true
            }
        }
        toggle.setOnCheckedChangeListener { _, on ->
            if (on) {
                if (store.time(day) != null) return@setOnCheckedChangeListener
                pickTime(null) { chosen ->
                    if (chosen == null) {
                        toggle.isChecked = false
                        return@pickTime
                    }
                    Remind.setDay(this, day, chosen)
                    time.text = chosen.format(CLOCK)
                    time.isEnabled = true
                }
            } else {
                Remind.setDay(this, day, null)
                time.text = "Off"
                time.isEnabled = false
            }
        }
        row.addView(label)
        row.addView(time)
        row.addView(toggle)
        days.addView(row)
    }

    private fun renderDues() {
        dueList.removeAllViews()
        val store = Store.open(this)
        val zone = ZoneId.systemDefault()
        val now = System.currentTimeMillis()
        for (due in store.dues().sorted()) {
            val whenDue = Instant.ofEpochMilli(due).atZone(zone)
            val block = LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(0, dp(12), 0, 0)
            }
            block.addView(TextView(this).apply { text = whenDue.format(DUE) })
            val wait = store.waitFor(due)
            if (wait != null && wait > now) {
                val until = Instant.ofEpochMilli(wait).atZone(zone)
                block.addView(TextView(this).apply { text = until.format(UNTIL) })
            }
            val actions = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
            actions.addView(Button(this).apply {
                text = "Later"
                setOnClickListener { onLater(due) }
            })
            actions.addView(Button(this).apply {
                text = "When"
                setOnClickListener { onWhen(due) }
            })
            actions.addView(Button(this).apply {
                text = "Open"
                setOnClickListener { openLog() }
            })
            block.addView(actions)
            dueList.addView(block)
        }
    }

    private fun onLater(due: Long) {
        val saved = Store.open(this).later
        if (saved != null) {
            Remind.later(this, due, saved)
            renderDues()
            return
        }
        pickDuration(null) { duration ->
            Store.open(this).later = duration
            showDuration(duration)
            Remind.later(this, due, duration)
            renderDues()
        }
    }

    private fun onWhen(due: Long) {
        pickTime(LocalTime.now()) { chosen ->
            if (chosen == null) return@pickTime
            Remind.whenAt(this, due, chosen)
            renderDues()
        }
    }

    private fun openLog() {
        saveAddress()
        val raw = Store.open(this).address
        startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(raw)))
    }

    private fun pickTime(current: LocalTime?, onPick: (LocalTime?) -> Unit) {
        val start = current ?: LocalTime.now()
        val dialog = TimePickerDialog(
            this,
            { _, hour, minute -> onPick(LocalTime.of(hour, minute)) },
            start.hour,
            start.minute,
            true,
        )
        dialog.setOnCancelListener { onPick(null) }
        dialog.show()
    }

    private fun pickDuration(current: Duration?, onSet: (Duration) -> Unit) {
        val start = current ?: Duration.ZERO
        val hours = NumberPicker(this).apply {
            minValue = 0
            maxValue = 23
            value = start.toHours().toInt().coerceIn(0, 23)
            wrapSelectorWheel = true
        }
        val minutes = NumberPicker(this).apply {
            minValue = 0
            maxValue = 59
            value = (start.toMinutes() % 60).toInt()
            wrapSelectorWheel = true
        }
        val row = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            val pad = dp(16)
            setPadding(pad, pad, pad, 0)
            addView(labeledPicker("Hours", hours))
            addView(labeledPicker("Minutes", minutes))
        }
        AlertDialog.Builder(this)
            .setTitle("How long")
            .setView(row)
            .setPositiveButton("OK") { _, _ ->
                val duration = Duration.ofHours(hours.value.toLong()).plusMinutes(minutes.value.toLong())
                if (duration.isZero) return@setPositiveButton
                onSet(duration)
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun labeledPicker(label: String, picker: NumberPicker): LinearLayout {
        val width = dp(96)
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(width, LinearLayout.LayoutParams.WRAP_CONTENT)
            addView(TextView(this@MainActivity).apply {
                text = label
                gravity = android.view.Gravity.CENTER_HORIZONTAL
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                )
            })
            addView(picker, LinearLayout.LayoutParams(width, dp(180)))
        }
    }

    private fun showExact() {
        val alarms = getSystemService(AlarmManager::class.java)
        val allowed = Build.VERSION.SDK_INT < Build.VERSION_CODES.S || alarms.canScheduleExactAlarms()
        val visibility = if (allowed) View.GONE else View.VISIBLE
        exact.visibility = visibility
        allowExact.visibility = visibility
    }

    private fun saveAddress() {
        val typed = address.text.toString().trim()
        Store.open(this).address = typed.ifEmpty { Store.ADDRESS }
        if (typed.isEmpty()) address.setText(Store.ADDRESS)
    }

    private fun showDuration(duration: Duration?) {
        if (duration == null || duration.isZero || duration.isNegative) {
            later.text = "Set"
            return
        }
        val hours = duration.toHours()
        val minutes = duration.toMinutes() % 60
        later.text = when {
            hours == 0L -> "$minutes min"
            minutes == 0L -> "$hours h"
            else -> "$hours h $minutes min"
        }
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    companion object {
        private val CLOCK: DateTimeFormatter = DateTimeFormatter.ofPattern("HH:mm")
        private val DUE: DateTimeFormatter = DateTimeFormatter.ofPattern("EEEE HH:mm")
        private val UNTIL: DateTimeFormatter = DateTimeFormatter.ofPattern("'until' EEE HH:mm")
    }
}
