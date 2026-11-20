package app.tether.design.components

import org.junit.Assert.assertEquals
import org.junit.Test

class BarFractionsTest {
    @Test
    fun allZeroValuesGiveZeroBars() {
        assertEquals(listOf(0f, 0f), barFractions(listOf(0L, 0L)))
    }

    @Test
    fun valuesScaleToTheMaximum() {
        assertEquals(listOf(0.5f, 1f), barFractions(listOf(50L, 100L)))
    }

    @Test
    fun singleValueIsFullHeight() {
        assertEquals(listOf(1f), barFractions(listOf(100L)))
    }

    @Test
    fun emptyListGivesEmptyList() {
        assertEquals(emptyList<Float>(), barFractions(emptyList()))
    }

    @Test
    fun negativeValuesNeverGiveNegativeBars() {
        assertEquals(listOf(0f, 1f), barFractions(listOf(-5L, 10L)))
        assertEquals(listOf(0f, 0f), barFractions(listOf(-5L, -1L)))
    }
}
