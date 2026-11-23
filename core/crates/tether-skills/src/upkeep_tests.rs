use super::{SkillScope, SkillState, SkillUsage, UpkeepAction, plan_upkeep};

fn usage(name: &str, last_used_at: Option<i64>, state: SkillState) -> SkillUsage {
    SkillUsage {
        name: name.into(),
        scope: SkillScope::User,
        last_used_at,
        created_at: 0,
        state,
    }
}

#[test]
fn upkeep_marks_stale_at_fourteen_days_and_archived_at_thirty() {
    let day = 86_400_000_i64;
    let skills = [
        usage("thirteen", Some(17 * day), SkillState::Active),
        usage("fourteen", Some(16 * day), SkillState::Active),
        usage("twenty_nine", Some(day), SkillState::Stale),
        usage("thirty", Some(0), SkillState::Stale),
    ];

    assert_eq!(
        plan_upkeep(&skills, 30 * day),
        vec![
            UpkeepAction {
                name: "fourteen".into(),
                from: SkillState::Active,
                to: SkillState::Stale,
            },
            UpkeepAction {
                name: "thirty".into(),
                from: SkillState::Stale,
                to: SkillState::Archived,
            },
        ]
    );
}

#[test]
fn recent_use_restores_active_and_bundled_skills_are_untouched() {
    let day = 86_400_000_i64;
    let mut used = usage("used", Some(29 * day), SkillState::Archived);
    used.scope = SkillScope::User;
    let bundled = SkillUsage {
        name: "bundled".into(),
        scope: SkillScope::Bundled,
        last_used_at: Some(0),
        created_at: 0,
        state: SkillState::Archived,
    };

    assert_eq!(
        plan_upkeep(&[used, bundled], 30 * day),
        vec![UpkeepAction {
            name: "used".into(),
            from: SkillState::Archived,
            to: SkillState::Active,
        }]
    );
}
