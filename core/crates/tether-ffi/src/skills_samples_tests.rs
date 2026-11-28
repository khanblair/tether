use super::{SkillsRuntime, SkillsRuntimeError};

#[test]
fn lists_uninstalled_samples_and_tracks_installation() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = SkillsRuntime::new(directory.path().join("skills").to_str().unwrap()).unwrap();

    let samples = runtime.sample_skills().unwrap();
    assert_eq!(samples.len(), 11);
    assert!(samples.iter().all(|sample| !sample.installed));
    assert!(samples.iter().any(|sample| sample.name == "tdd"));

    runtime.install_sample_skill("tdd".into()).unwrap();
    let tdd = runtime
        .sample_skills()
        .unwrap()
        .into_iter()
        .find(|sample| sample.name == "tdd")
        .unwrap();
    assert!(tdd.installed);
    assert_eq!(tdd.file_count, 3);
    assert!(
        runtime
            .list()
            .unwrap()
            .iter()
            .any(|skill| skill.folder == "tdd" && skill.valid)
    );
}

#[test]
fn installing_twice_succeeds_and_unknown_samples_are_not_found() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = SkillsRuntime::new(directory.path().to_str().unwrap()).unwrap();
    runtime.install_sample_skill("tdd".into()).unwrap();
    runtime.install_sample_skill("tdd".into()).unwrap();
    assert!(matches!(
        runtime.install_sample_skill("missing".into()),
        Err(SkillsRuntimeError::NotFound)
    ));
}

#[test]
fn notice_contains_the_sample_attributions() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = SkillsRuntime::new(directory.path().to_str().unwrap()).unwrap();
    let notice = runtime.sample_skills_notice();
    assert!(notice.contains("Matt Pocock"));
    assert!(notice.contains("Julius Brussee"));
}
