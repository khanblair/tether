//! Pure planning for prune-only skill upkeep.

const DAY_MS: i64 = 86_400_000;
const STALE_AFTER_MS: i64 = 14 * DAY_MS;
const ARCHIVE_AFTER_MS: i64 = 30 * DAY_MS;

/// The location that owns a skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillScope {
    /// A private user skill.
    User,
    /// A trusted project skill.
    Project,
    /// A skill shipped with the app.
    Bundled,
}

/// The file-backed upkeep state of a skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillState {
    /// The skill is available.
    Active,
    /// The skill has not been used recently.
    Stale,
    /// The skill is in the archive folder.
    Archived,
}

/// Usage data used to plan one upkeep run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillUsage {
    /// Skill name.
    pub name: String,
    /// Skill location.
    pub scope: SkillScope,
    /// Last use time in Unix milliseconds.
    pub last_used_at: Option<i64>,
    /// Install or creation time in Unix milliseconds.
    pub created_at: i64,
    /// Current file-backed state.
    pub state: SkillState,
}

/// One state change planned by upkeep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpkeepAction {
    /// Skill name.
    pub name: String,
    /// State before the run.
    pub from: SkillState,
    /// State after the run.
    pub to: SkillState,
}

/// Plans stale and archived transitions without deleting any skill.
#[must_use]
pub fn plan_upkeep(skills: &[SkillUsage], now_ms: i64) -> Vec<UpkeepAction> {
    skills
        .iter()
        .filter(|skill| skill.scope != SkillScope::Bundled)
        .filter_map(|skill| action_for(skill, now_ms))
        .collect()
}

fn action_for(skill: &SkillUsage, now_ms: i64) -> Option<UpkeepAction> {
    let reference = skill.last_used_at.unwrap_or(skill.created_at);
    let unused = now_ms.saturating_sub(reference);
    let to = if skill
        .last_used_at
        .is_some_and(|last| now_ms.saturating_sub(last) < STALE_AFTER_MS)
    {
        SkillState::Active
    } else if unused >= ARCHIVE_AFTER_MS {
        SkillState::Archived
    } else if unused >= STALE_AFTER_MS {
        SkillState::Stale
    } else {
        SkillState::Active
    };
    (to != skill.state).then(|| UpkeepAction {
        name: skill.name.clone(),
        from: skill.state,
        to,
    })
}
