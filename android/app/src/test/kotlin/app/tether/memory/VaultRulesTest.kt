package app.tether.memory

import app.tether.memory.BodyPart.BodyLink
import app.tether.memory.BodyPart.BodyText
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class VaultRulesTest {
    @Test
    fun a_body_with_no_link_is_one_text_part() {
        assertEquals(listOf(BodyText("Plain")), parseBody("Plain"))
    }

    @Test
    fun the_body_splits_into_text_and_links_with_aliases_and_headings() {
        val parts = parseBody("See [[Auth session design]] and [[Commit style#Tests|the tests]].")
        assertEquals(
            listOf(
                BodyText("See "),
                BodyLink(
                    "Auth session design",
                    null,
                    "Auth session design",
                    resolved = false,
                    resolvedPath = null,
                ),
                BodyText(" and "),
                BodyLink(
                    "Commit style",
                    "Tests",
                    "the tests",
                    resolved = false,
                    resolvedPath = null,
                ),
                BodyText("."),
            ),
            parts,
        )
    }

    @Test
    fun a_heading_without_an_alias_is_shown_after_the_target() {
        val link = parseBody("[[Commit style#Tests]]").single() as BodyLink
        assertEquals("Commit style > Tests", link.label)
    }

    @Test
    fun a_link_is_resolved_only_when_the_core_lists_its_target_as_found() {
        val outgoing =
            listOf(
                VaultLink(
                    from = "notes/auth.md",
                    target = "Auth session design",
                    heading = null,
                    alias = null,
                    resolved = true,
                    resolvedPath = "notes/Auth session design.md",
                ),
            )
        val parts =
            resolveBodyLinks(parseBody("[[Auth session design]] and [[Token rotation]]"), outgoing)
        val links = parts.filterIsInstance<BodyLink>()
        assertEquals(true, links[0].resolved)
        assertEquals("notes/Auth session design.md", links[0].resolvedPath)
        assertEquals(false, links[1].resolved)
        assertNull(links[1].resolvedPath)
    }

    @Test
    fun a_link_the_core_marks_found_but_gives_no_path_is_missing() {
        val outgoing =
            listOf(
                VaultLink(
                    "notes/auth.md",
                    "Auth",
                    null,
                    null,
                    resolved = true,
                    resolvedPath = null,
                ),
            )
        val link = resolveBodyLinks(parseBody("[[Auth]]"), outgoing).single() as BodyLink
        assertEquals(false, link.resolved)
        assertNull(link.resolvedPath)
    }

    @Test
    fun the_frontmatter_is_left_out_of_the_body() {
        val source = "---\ndescription: Auth\ntags: [auth]\n---\n\n# Title\nBody"
        assertEquals("# Title\nBody", noteBody(source))
    }

    @Test
    fun a_note_without_frontmatter_has_its_whole_source_as_body() {
        assertEquals("# Title", noteBody("# Title"))
    }

    @Test
    fun a_relative_markdown_path_inside_the_vault_is_accepted() {
        assertNull(vaultPathProblem("notes/auth.md"))
    }

    @Test
    fun paths_outside_the_vault_rules_are_refused() {
        listOf(
            "auth",
            "notes/auth.txt",
            ".md",
            ".obsidian/app.md",
            "notes/.hidden.md",
            "../x.md",
            "/abs.md",
            "a//b.md",
        ).forEach {
            assertEquals(
                "$it should be refused",
                AreaProblem.InvalidPath,
                vaultPathProblem(it),
            )
        }
    }
}
