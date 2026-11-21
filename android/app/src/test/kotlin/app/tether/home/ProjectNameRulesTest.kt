package app.tether.home

import org.junit.Assert.assertEquals
import org.junit.Test

class ProjectNameRulesTest {
    @Test
    fun an_empty_or_blank_name_is_empty_and_shows_no_message() {
        assertEquals(ProjectNameCheck.Empty, checkProjectName(""))
        assertEquals(ProjectNameCheck.Empty, checkProjectName("   "))
    }

    @Test
    fun one_character_is_the_shortest_valid_name() {
        assertEquals(ProjectNameCheck.Valid("a"), checkProjectName("a"))
    }

    @Test
    fun eighty_characters_is_the_longest_valid_name() {
        val name = "a".repeat(80)
        assertEquals(ProjectNameCheck.Valid(name), checkProjectName(name))
    }

    @Test
    fun eighty_one_characters_is_too_long() {
        assertEquals(
            ProjectNameCheck.Invalid(ProjectNameProblem.TooLong),
            checkProjectName("a".repeat(81)),
        )
    }

    @Test
    fun length_counts_code_points_so_eighty_emoji_are_valid() {
        val name = "😀".repeat(80)
        assertEquals(ProjectNameCheck.Valid(name), checkProjectName(name))
    }

    @Test
    fun a_newline_inside_the_name_is_a_control_character() {
        assertEquals(
            ProjectNameCheck.Invalid(ProjectNameProblem.ControlCharacter),
            checkProjectName("api\nserver"),
        )
    }

    @Test
    fun a_tab_inside_the_name_is_a_control_character() {
        assertEquals(
            ProjectNameCheck.Invalid(ProjectNameProblem.ControlCharacter),
            checkProjectName("api\tserver"),
        )
    }

    @Test
    fun leading_and_trailing_spaces_and_tabs_are_trimmed_away() {
        assertEquals(ProjectNameCheck.Valid("blog"), checkProjectName("  \tblog\t  "))
    }

    @Test
    fun a_name_of_eighty_characters_with_spaces_around_it_is_valid() {
        val name = "a".repeat(80)
        assertEquals(ProjectNameCheck.Valid(name), checkProjectName("  $name  "))
    }

    @Test
    fun length_is_reported_before_a_control_character() {
        assertEquals(
            ProjectNameCheck.Invalid(ProjectNameProblem.TooLong),
            checkProjectName("a".repeat(80) + "\n" + "b"),
        )
    }
}
