package app.tether.session.agent

import app.tether.core.generated.PermissionModeView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

class FakeAgentLoopGatewayTest {
    @Test fun recordsEachCallInOrder() {
        val fake = FakeAgentLoopGateway()

        fake.approve("s1", "t1", always = true)
        fake.runPlan("s1", PermissionModeView.AUTO)

        assertEquals(listOf("approve:s1,t1,true", "runPlan:s1,AUTO"), fake.log)
    }

    @Test fun returnsTheScriptedValues() {
        val fake = FakeAgentLoopGateway()
        val approval = ApprovalUi("t1", "run", "npm test")
        fake.approvals = listOf(approval)

        assertEquals(listOf(approval), fake.pendingApprovals("s1"))
        assertNull(fake.pendingQuestion("s1"))
    }

    @Test fun failNextThrowsOnceThenClears() {
        val fake = FakeAgentLoopGateway()
        fake.failNext = AgentLoopException(AgentLoopFailure.Busy)

        assertThrows(AgentLoopException::class.java) { fake.resume("s1") }
        fake.resume("s1")

        assertEquals(listOf("resume:s1", "resume:s1"), fake.log)
    }
}
