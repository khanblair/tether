use super::skill_specs;

#[test]
fn skill_manage_describes_the_agent_skills_file_format() {
    let description = skill_specs()
        .into_iter()
        .find(|spec| spec.name == "skill_manage")
        .map(|spec| spec.description)
        .unwrap();

    assert!(description.contains("name"));
    assert!(description.contains("description"));
    assert!(description.contains("SKILL.md"));
    assert!(description.chars().count() < 600);
}
