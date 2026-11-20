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
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.tether.R
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
    val maximum = LocalConfiguration.current.screenHeightDp.dp * 0.4f
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
                        val text = input.trim()
                        if (text.isNotEmpty()) {
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
