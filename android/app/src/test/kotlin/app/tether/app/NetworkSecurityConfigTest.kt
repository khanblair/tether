package app.tether.app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class NetworkSecurityConfigTest {
    @Test
    fun only_loopback_is_permitted_cleartext() {
        val xml = File("src/main/res/xml/network_security_config.xml").readText()
        assertTrue(xml.contains("<base-config cleartextTrafficPermitted=\"false\" />"))
        assertTrue(xml.contains("<domain-config cleartextTrafficPermitted=\"true\">"))
        assertTrue(xml.contains("<domain includeSubdomains=\"false\">127.0.0.1</domain>"))
        assertFalse(xml.contains("<domain includeSubdomains=\"true\">"))
        assertFalse(
            Regex("<domain[^>]*>[^<]+</domain>")
                .findAll(xml)
                .any { it.value.contains("127.0.0.1") == false },
        )
    }
}
