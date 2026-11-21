package app.tether.session

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RewindScopeTest {
    @Test
    fun the_both_scope_restores_files_and_conversation() {
        assertTrue(RewindScope.FILES_AND_CONVERSATION.restoresFiles)
        assertTrue(RewindScope.FILES_AND_CONVERSATION.restoresConversation)
    }

    @Test
    fun the_files_scope_restores_files_only() {
        assertTrue(RewindScope.FILES.restoresFiles)
        assertFalse(RewindScope.FILES.restoresConversation)
    }

    @Test
    fun the_conversation_scope_restores_the_conversation_only() {
        assertFalse(RewindScope.CONVERSATION.restoresFiles)
        assertTrue(RewindScope.CONVERSATION.restoresConversation)
    }

    @Test
    fun only_scopes_that_touch_files_need_a_confirmation() {
        assertTrue(RewindScope.FILES_AND_CONVERSATION.needsConfirmation)
        assertTrue(RewindScope.FILES.needsConfirmation)
        assertFalse(RewindScope.CONVERSATION.needsConfirmation)
    }
}
