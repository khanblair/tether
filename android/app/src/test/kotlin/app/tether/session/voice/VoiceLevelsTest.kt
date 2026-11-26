package app.tether.session.voice

import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Test

class VoiceLevelsTest {
    @Test fun newestLevelGoesLastAndTheOldestDrops() {
        val before = List(VOICE_BARS) { it / 10f }

        val after = nextLevels(before, 0.75f)

        assertEquals(VOICE_BARS, after.size)
        assertEquals(before.drop(1) + 0.75f, after)
    }

    @Test fun shortHistoryIsPaddedWithSilence() {
        val after = nextLevels(emptyList(), 0.5f)

        assertEquals(List(VOICE_BARS - 1) { 0f } + 0.5f, after)
    }

    @Test fun levelsOutsideZeroToOneAreClamped() {
        assertEquals(1f, nextLevels(emptyList(), 3f).last(), 0f)
        assertEquals(0f, nextLevels(emptyList(), -1f).last(), 0f)
    }

    @Test fun rmsMapsToZeroToOne() {
        assertEquals(0f, levelFromRms(-2f), 0.0001f)
        assertEquals(0.5f, levelFromRms(4f), 0.0001f)
        assertEquals(1f, levelFromRms(10f), 0.0001f)
        assertEquals(1f, levelFromRms(40f), 0f)
        assertEquals(0f, levelFromRms(-30f), 0f)
    }

    @Test fun barHeightRunsFromEightToFortyDp() {
        assertEquals(8f, barHeight(0f).value, 0.0001f)
        assertEquals(24f, barHeight(0.5f).value, 0.0001f)
        assertEquals(40f, barHeight(1f).value, 0.0001f)
        assertEquals(8.dp, barHeight(-1f))
    }
}
