package app.tether.onboarding

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.settings.SettingsStore
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/** The steps of the first-run flow, in the order the user sees them. */
enum class OnboardingStep {
    Welcome,
    AddKey,
    Workspace,
    OpenProject,
}

/** Where the flow sends the user when it ends. */
enum class OnboardingExit {
    /** Skipped, or finished without creating a project. Opens Home. */
    Home,

    /** New empty project was chosen. Opens the new project screen. */
    NewProject,

    /** Clone from GitHub was chosen. Opens the clone screen. */
    Clone,

    /** Import a zip was chosen. Opens the zip import screen. */
    Import,
}

/** What the onboarding screens show. [exit] is set once the flow has ended. */
@Immutable
data class OnboardingUiState(
    val step: OnboardingStep = OnboardingStep.Welcome,
    val name: String = "",
    val exit: OnboardingExit? = null,
)

/**
 * Holds the first-run flow's step and the optional display name. The flow is done once the user
 * skips it or chooses a project, and then [SettingsStore] remembers that it is done.
 */
class OnboardingViewModel(
    private val settings: SettingsStore,
) : ViewModel() {
    private val mutableState = MutableStateFlow(OnboardingUiState(name = settings.profileName()))

    /** The screen state. */
    val state: StateFlow<OnboardingUiState> = mutableState.asStateFlow()

    /** The display name field changed. */
    fun onNameChange(text: String) {
        mutableState.update { it.copy(name = text) }
    }

    /** Moves to the next step. Leaving the welcome step saves the trimmed display name. */
    fun next() {
        val current = mutableState.value
        if (current.step == OnboardingStep.Welcome) settings.saveProfileName(current.name.trim())
        val following = OnboardingStep.entries.getOrNull(current.step.ordinal + 1) ?: return
        mutableState.update { it.copy(step = following) }
    }

    /** Goes back one step. The first step has no previous one, so nothing happens there. */
    fun back() {
        val previous =
            OnboardingStep.entries.getOrNull(mutableState.value.step.ordinal - 1) ?: return
        mutableState.update { it.copy(step = previous) }
    }

    /** The user skipped the flow. It is marked done, and the app opens Home. */
    fun skip() {
        finish(OnboardingExit.Home)
    }

    /** The user chose New empty project. The flow is done, and the app opens that screen. */
    fun openNewProject() {
        finish(OnboardingExit.NewProject)
    }

    /** The user chose Clone from GitHub. The flow is done, and the app opens the clone screen. */
    fun openClone() {
        finish(OnboardingExit.Clone)
    }

    /** The user chose Import a zip. The flow is done, and the app opens the import screen. */
    fun openImport() {
        finish(OnboardingExit.Import)
    }

    private fun finish(exit: OnboardingExit) {
        settings.saveOnboarded(true)
        mutableState.update { it.copy(exit = exit) }
    }

    companion object {
        /** Builds the flow's state holder with the app's settings. */
        fun factory(settings: SettingsStore) =
            viewModelFactory { initializer { OnboardingViewModel(settings) } }
    }
}
