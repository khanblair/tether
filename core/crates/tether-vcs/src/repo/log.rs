use std::collections::BTreeMap;

use super::RepoError;

#[derive(Clone)]
struct LogNode {
    parents: Vec<gix::hash::ObjectId>,
    children: usize,
    time: i64,
    order: usize,
}

pub(super) fn log_order(
    repo: &gix::Repository,
    head: gix::hash::ObjectId,
) -> Result<Vec<gix::hash::ObjectId>, RepoError> {
    let mut nodes = BTreeMap::new();
    let mut pending = vec![head];
    while let Some(id) = pending.pop() {
        if nodes.contains_key(&id) {
            continue;
        }
        let commit = repo.find_commit(id).map_err(|_| RepoError::Git)?;
        let parents: Vec<_> = commit.parent_ids().map(gix::Id::detach).collect();
        let time = commit
            .committer()
            .map_err(|_| RepoError::Git)?
            .time()
            .map_err(|_| RepoError::Git)?
            .seconds;
        let order = nodes.len();
        pending.extend(parents.iter().rev().copied());
        nodes.insert(
            id,
            LogNode {
                parents,
                children: 0,
                time,
                order,
            },
        );
    }
    let edges: Vec<_> = nodes
        .values()
        .flat_map(|node| node.parents.iter().copied())
        .collect();
    for parent in edges {
        if let Some(parent_node) = nodes.get_mut(&parent) {
            parent_node.children += 1;
        }
    }
    emit_log_order(nodes, head)
}

fn emit_log_order(
    mut nodes: BTreeMap<gix::hash::ObjectId, LogNode>,
    head: gix::hash::ObjectId,
) -> Result<Vec<gix::hash::ObjectId>, RepoError> {
    let mut ready = vec![head];
    let mut order = Vec::with_capacity(nodes.len());
    while !ready.is_empty() {
        let index = ready
            .iter()
            .enumerate()
            .max_by_key(|(_, id)| node_key(nodes.get(*id)))
            .map(|(index, _)| index)
            .ok_or(RepoError::Git)?;
        let id = ready.swap_remove(index);
        let node = nodes.remove(&id).ok_or(RepoError::Git)?;
        order.push(id);
        for parent in node.parents {
            release_parent(&mut nodes, &mut ready, parent);
        }
    }
    Ok(order)
}

fn node_key(node: Option<&LogNode>) -> (usize, i64) {
    node.map_or((usize::MIN, i64::MIN), |node| {
        (usize::MAX - node.order, node.time)
    })
}

fn release_parent(
    nodes: &mut BTreeMap<gix::hash::ObjectId, LogNode>,
    ready: &mut Vec<gix::hash::ObjectId>,
    parent: gix::hash::ObjectId,
) {
    let Some(parent_node) = nodes.get_mut(&parent) else {
        return;
    };
    parent_node.children = parent_node.children.saturating_sub(1);
    if parent_node.children == 0 {
        ready.push(parent);
    }
}
