use tether_types::{Provider, SessionId, UsageSnapshot};

use super::*;

fn project(store: &Store, id: &str, name: &str) -> tether_types::ProjectId {
    store
        .conn
        .execute(
            "INSERT INTO projects (id, name, path, created_at) VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![id, name, format!("path-{id}")],
        )
        .unwrap();
    tether_types::ProjectId::new(id)
}

fn session(store: &mut Store, project: &tether_types::ProjectId, id: &str) -> SessionId {
    store
        .conn
        .execute(
            "INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
             VALUES (?1, ?2, 'openai', 'model', 'ask', 'idle', 1, 1)",
            rusqlite::params![id, project.as_str()],
        )
        .unwrap();
    SessionId::new(id)
}

#[test]
fn response_usage_updates_session_and_daily_rows_in_one_total() {
    let mut store = Store::open_in_memory().unwrap();
    let project = project(&store, "p1", "Project");
    let id = session(&mut store, &project, "s1");
    store
        .record_response_usage(&ResponseUsageRecord {
            session_id: id.clone(),
            provider: Provider::OpenAi,
            model_id: "model".into(),
            project_id: project.as_str().into(),
            day: "2026-11-21".into(),
            usage: UsageSnapshot {
                tokens_in: 10,
                tokens_out: 2,
                cost_micros: 30,
            },
            price_known: true,
            now_ms: 2,
        })
        .unwrap();
    store
        .record_response_usage(&ResponseUsageRecord {
            session_id: id.clone(),
            provider: Provider::OpenAi,
            model_id: "model".into(),
            project_id: project.as_str().into(),
            day: "2026-11-21".into(),
            usage: UsageSnapshot {
                tokens_in: 4,
                tokens_out: 1,
                cost_micros: 20,
            },
            price_known: false,
            now_ms: 3,
        })
        .unwrap();

    assert_eq!(store.session_usage(&id).unwrap().cost_micros, 50);
    assert!(!store.session_price_known(&id).unwrap());
    assert_eq!(store.usage_totals("2026-11-21").unwrap().tokens_in, 14);
}

#[test]
fn daily_write_failure_rolls_back_session_total() {
    let mut store = Store::open_in_memory().unwrap();
    let project = project(&store, "p1", "Project");
    let id = session(&mut store, &project, "s1");
    let result = store.record_response_usage(&ResponseUsageRecord {
        session_id: id.clone(),
        provider: Provider::OpenAi,
        model_id: "model".into(),
        project_id: project.as_str().into(),
        day: "not-a-day".into(),
        usage: UsageSnapshot {
            tokens_in: 10,
            tokens_out: 2,
            cost_micros: 30,
        },
        price_known: true,
        now_ms: 2,
    });
    assert!(result.is_err());
    assert_eq!(store.session_usage(&id).unwrap(), UsageSnapshot::default());
    assert_eq!(
        store.usage_totals("2026-11-21").unwrap(),
        UsageTotals::default()
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the history fixture covers accumulation and deleted-project retention"
)]
fn usage_history_accumulates_across_sessions_and_keeps_deleted_projects() {
    let mut store = Store::open_in_memory().unwrap();
    let first_project = project(&store, "p1", "First");
    let second_project = project(&store, "p2", "Second");
    let first = session(&mut store, &first_project, "s1");
    let second = session(&mut store, &second_project, "s2");
    let usage = UsageSnapshot {
        tokens_in: 1,
        tokens_out: 2,
        cost_micros: 3,
    };
    store
        .record_response_usage(&ResponseUsageRecord {
            session_id: first,
            provider: Provider::OpenAi,
            model_id: "model".into(),
            project_id: "p1".into(),
            day: "2026-11-20".into(),
            usage,
            price_known: true,
            now_ms: 2,
        })
        .unwrap();
    store
        .record_response_usage(&ResponseUsageRecord {
            session_id: second,
            provider: Provider::OpenAi,
            model_id: "model".into(),
            project_id: "p2".into(),
            day: "2026-11-21".into(),
            usage,
            price_known: true,
            now_ms: 3,
        })
        .unwrap();
    store
        .conn
        .execute("DELETE FROM projects WHERE id = 'p2'", [])
        .unwrap();

    let days = store.usage_by_day("2026-11-20", "2026-11-21").unwrap();
    assert_eq!(
        days.iter().map(|row| row.day.as_str()).collect::<Vec<_>>(),
        ["2026-11-20", "2026-11-21"]
    );
    let projects = store.usage_by_project("2026-11-20", "2026-11-21").unwrap();
    assert_eq!(projects[1].project_id, "p2");
    assert_eq!(projects[1].project_name, "");
    assert_eq!(
        store
            .usage_windows("2026-11-21")
            .unwrap()
            .last_7_days
            .cost_micros,
        6
    );
}

#[test]
fn budgets_validate_limits_and_overrides() {
    let mut store = Store::open_in_memory().unwrap();
    let project = project(&store, "p1", "Project");
    let id = session(&mut store, &project, "s1");
    assert!(matches!(
        store.set_budgets(
            BudgetSettings {
                daily_limit_micros: Some(9_999),
                session_limit_micros: None
            },
            1
        ),
        Err(StoreError::InvalidBudget)
    ));
    store
        .set_budgets(
            BudgetSettings {
                daily_limit_micros: Some(20_000),
                session_limit_micros: Some(30_000),
            },
            1,
        )
        .unwrap();
    assert_eq!(store.budgets().unwrap().daily_limit_micros, Some(20_000));
    store.set_session_budget(&id, Some(10_000)).unwrap();
    assert_eq!(store.session_budget(&id).unwrap(), Some((10_000, None)));
    store.set_session_budget(&id, None).unwrap();
    assert_eq!(store.session_budget(&id).unwrap(), None);
}

#[test]
fn usage_day_offset_crosses_month_and_year_boundaries() {
    assert_eq!(usage_day_offset("2026-01-01", -1).unwrap(), "2025-12-31");
    assert_eq!(usage_day_offset("2026-03-01", -1).unwrap(), "2026-02-28");
}
