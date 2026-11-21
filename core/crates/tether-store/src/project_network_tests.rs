use tether_types::ProjectId;

use super::{Store, StoreError};

#[test]
fn project_network_defaults_off_and_can_be_changed() {
    let mut store = Store::open_in_memory().unwrap();
    let project = store.create_project("demo", 1).unwrap();

    assert!(!store.project_network(&project.id).unwrap());

    store.set_project_network(&project.id, true, 2).unwrap();
    assert!(store.project_network(&project.id).unwrap());
}

#[test]
fn project_network_row_cascades_when_project_is_deleted() {
    let mut store = Store::open_in_memory().unwrap();
    let project = store.create_project("demo", 1).unwrap();
    store.set_project_network(&project.id, true, 2).unwrap();

    store.delete_project(&project.id).unwrap();

    assert!(matches!(
        store.project_network(&ProjectId::new(project.id.as_str())),
        Err(StoreError::MissingProject)
    ));
}
