package log.session

import java.time.DayOfWeek
import java.time.Duration
import java.time.Instant
import java.time.LocalTime
import java.time.ZonedDateTime

object Schedule {
    fun nextWeekly(day: DayOfWeek, time: LocalTime, now: ZonedDateTime): ZonedDateTime {
        val daysAhead = (day.value - now.dayOfWeek.value + 7) % 7
        var at = now.toLocalDate().plusDays(daysAhead.toLong()).atTime(time).atZone(now.zone)
        if (!at.isAfter(now)) at = at.plusWeeks(1)
        return at
    }

    fun latestPast(day: DayOfWeek, time: LocalTime, now: ZonedDateTime): ZonedDateTime =
        nextWeekly(day, time, now).minusWeeks(1)

    fun whenWait(clock: LocalTime, now: ZonedDateTime): ZonedDateTime {
        val today = now.toLocalDate().atTime(clock).atZone(now.zone)
        return if (today.isAfter(now)) today else today.plusDays(1)
    }

    fun laterWait(duration: Duration, now: ZonedDateTime): ZonedDateTime = now.plus(duration)

    fun clearedBy(due: Instant, createdAt: List<Instant>): Boolean =
        createdAt.any { it.isAfter(due) }
}
