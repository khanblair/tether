package app.tether.session

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.tokens.TetherRatio
import app.tether.design.tokens.TetherSpace

/** The session input. The field grows with text, capped at 40% of the display height. */
@Composable
fun Composer(
    modelId: String,
    running: Boolean,
    onSend: (String) -> Unit,
    onStop: () -> Unit,
) {
    var input by rememberSaveable { mutableStateOf("") }
    val windowHeight = LocalWindowInfo.current.containerSize.height
    val maximum = with(LocalDensity.current) { windowHeight.toDp() * TetherRatio.composerMax }
    Column(modifier = Modifier.fillMaxWidth().padding(TetherSpace.space4)) {
        OutlinedTextField(
            value = input,
            onValueChange = { input = it },
            label = { Text(stringResource(R.string.composer_hint)) },
            modifier = Modifier.fillMaxWidth().heightIn(max = maximum),
            maxLines = Int.MAX_VALUE,
        )
        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
            Text(stringResource(R.string.composer_attachments))
            Text(modelId)
            if (running) {
                Button(onClick = onStop) { Text(stringResource(R.string.composer_stop)) }
            } else {
                Button(
                    onClick = {
                        val text = outgoingText(input)
                        if (text != null) {
                            onSend(text)
                            input = ""
                        }
                    },
                    enabled = input.isNotBlank(),
                ) { Text(stringResource(R.string.composer_send)) }
            }
        }
    }
}

/** Blank input never starts a turn. Preserve line breaks inside a message. */
internal fun outgoingText(input: String): String? = input.trim().takeIf(String::isNotEmpty)
