package app.tether.session

import app.tether.core.generated.ReasoningEffortView

/**
 * How much a model should think before it answers: Low, Medium, or High. A null level means the
 * model's own default, and nothing is sent. The core sends the level only to models that report
 * reasoning support.
 */
enum class ThinkingEffort { LOW, MEDIUM, HIGH }

internal fun ThinkingEffort.toCore(): ReasoningEffortView =
    when (this) {
        ThinkingEffort.LOW -> ReasoningEffortView.LOW
        ThinkingEffort.MEDIUM -> ReasoningEffortView.MEDIUM
        ThinkingEffort.HIGH -> ReasoningEffortView.HIGH
    }

internal fun ReasoningEffortView.toThinking(): ThinkingEffort =
    when (this) {
        ReasoningEffortView.LOW -> ThinkingEffort.LOW
        ReasoningEffortView.MEDIUM -> ThinkingEffort.MEDIUM
        ReasoningEffortView.HIGH -> ThinkingEffort.HIGH
    }
