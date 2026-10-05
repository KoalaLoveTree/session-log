package log.session

import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/** The phone's log. Notes are saved here, then sent on the next open. */
class LogActivity : AppCompatActivity() {
    private lateinit var draft: EditText
    private lateinit var add: Button
    private lateinit var toggle: Button
    private lateinit var status: TextView
    private lateinit var empty: TextView
    private lateinit var rows: LinearLayout
    private var deleted = false
    private var editing: String? = null
    private val buffers = HashMap<String, String>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_log)
        draft = findViewById(R.id.draft)
        add = findViewById(R.id.add)
        toggle = findViewById(R.id.toggle)
        status = findViewById(R.id.sync_status)
        empty = findViewById(R.id.empty)
        rows = findViewById(R.id.rows)
        add.setOnClickListener {
            if (!Copy.add(this, draft.text.toString())) return@setOnClickListener
            draft.text.clear()
            show()
        }
        toggle.setOnClickListener {
            deleted = !deleted
            toggle.text = if (deleted) "Log" else "Deleted"
            show()
        }
    }

    override fun onResume() {
        super.onResume()
        show()
        Sync.onOpen(this) { message ->
            status.text = message.orEmpty()
            show()
        }
    }

    private fun show() {
        val notes = Copy.notes(this, deleted)
        val asks = Copy.asks(this).associateBy { it.id }
        draft.visibility = if (deleted) View.GONE else View.VISIBLE
        add.visibility = if (deleted) View.GONE else View.VISIBLE
        empty.text = if (deleted) "No deleted entries." else "No entries yet."
        empty.visibility = if (notes.isEmpty()) View.VISIBLE else View.GONE
        rows.removeAllViews()
        for (note in notes) rows.addView(row(note, asks[note.id]))
    }

    private fun row(note: Copy.Note, ask: Copy.Purge?): LinearLayout {
        val block = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(12), 0, 0)
        }
        block.addView(TextView(this).apply { text = whenWritten(note.createdAt) })
        if (ask != null) {
            block.addView(TextView(this).apply {
                text = question(note)
                setPadding(0, dp(4), 0, 0)
            })
            val actions = actions()
            actions.addView(button("Remove") {
                Copy.accept(this, note.id)
                buffers.remove(note.id)
                if (editing == note.id) editing = null
                show()
            })
            actions.addView(button("Keep") {
                Sync.decline(this, note.id) { message ->
                    status.text = message.orEmpty()
                    show()
                }
            })
            block.addView(actions)
        }
        if (note.otherBody != null) {
            block.addView(TextView(this).apply {
                text = note.body
                setPadding(0, dp(8), 0, 0)
            })
            block.addView(TextView(this).apply {
                text = "Other"
                setPadding(0, dp(8), 0, 0)
            })
            block.addView(TextView(this).apply { text = note.otherBody })
            val editor = editor(note.id, note.body)
            block.addView(editor)
            block.addView(button("Save") {
                if (!Copy.save(this, note.id, editor.text.toString())) return@button
                buffers.remove(note.id)
                if (editing == note.id) editing = null
                if (deleted) {
                    deleted = false
                    toggle.text = "Deleted"
                }
                show()
            })
            return block
        }
        block.addView(TextView(this).apply {
            text = note.body
            setPadding(0, dp(4), 0, 0)
        })
        if (ask != null) return block
        val actions = actions()
        if (deleted) {
            actions.addView(button("Restore") {
                Copy.restore(this, note.id)
                show()
            })
            actions.addView(button("Purge") {
                confirm {
                    Copy.purge(this, note.id)
                    show()
                }
            })
        } else if (editing == note.id) {
            val editor = editor(note.id, note.body)
            block.addView(editor)
            actions.addView(button("Save") {
                if (!Copy.save(this, note.id, editor.text.toString())) return@button
                buffers.remove(note.id)
                editing = null
                show()
            })
            actions.addView(button("Cancel") {
                buffers.remove(note.id)
                editing = null
                show()
            })
        } else {
            actions.addView(button("Edit") {
                editing = note.id
                buffers[note.id] = note.body
                show()
            })
            actions.addView(button("Delete") {
                confirm {
                    Copy.softDelete(this, note.id)
                    show()
                }
            })
        }
        block.addView(actions)
        return block
    }

    private fun question(note: Copy.Note): String {
        val updated = runCatching { Instant.parse(note.updatedAt) }.getOrNull()
        val synced = note.syncedAt?.let { runCatching { Instant.parse(it) }.getOrNull() }
        return if (updated != null && updated == synced) {
            "The PC purged this. Remove it here too?"
        } else {
            "The PC purged this, and this copy changed after the last sync. Remove it here too?"
        }
    }

    private fun editor(id: String, initial: String): EditText {
        return EditText(this).apply {
            setText(buffers.getOrPut(id) { initial })
            minLines = 3
            gravity = android.view.Gravity.TOP
            inputType = android.text.InputType.TYPE_CLASS_TEXT or
                android.text.InputType.TYPE_TEXT_FLAG_MULTI_LINE
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit
                override fun afterTextChanged(s: Editable?) {
                    buffers[id] = s?.toString().orEmpty()
                }
            })
        }
    }

    private fun actions(): LinearLayout {
        return LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
    }

    private fun button(label: String, onClick: () -> Unit): Button {
        return Button(this).apply {
            text = label
            setOnClickListener { onClick() }
        }
    }

    private fun confirm(onYes: () -> Unit) {
        AlertDialog.Builder(this)
            .setTitle("Are you sure?")
            .setPositiveButton("OK") { _, _ -> onYes() }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun whenWritten(iso: String): String {
        val instant = runCatching { Instant.parse(iso) }.getOrNull() ?: return iso
        return WHEN.format(instant.atZone(ZoneId.systemDefault()))
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    companion object {
        private val WHEN: DateTimeFormatter = DateTimeFormatter.ofPattern("EEE yyyy-MM-dd HH:mm:ss")
    }
}
