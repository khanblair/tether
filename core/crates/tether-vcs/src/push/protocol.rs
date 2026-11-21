use super::{
    BTreeMap, BTreeSet, CancellationToken, PushError, PushPhase, PushProgress, PushReport,
    PushRequest, RefResult, Repo, Transport, ZERO, validate_branch_name,
};
use std::io::Write;

pub(super) fn advertise(
    request: &PushRequest,
    transport: &dyn Transport,
    progress: &dyn PushProgress,
) -> Result<BTreeMap<String, String>, PushError> {
    progress.update(PushPhase::Advertising, 0, None);
    let url = format!(
        "{}{}",
        receive_url(&request.remote_url),
        "/info/refs?service=git-receive-pack"
    );
    let bytes = transport.get(
        &url,
        BTreeMap::from([(
            String::from("accept"),
            String::from("application/x-git-receive-pack-advertisement"),
        )]),
    )?;
    parse_advertisement(&bytes)
}

pub(super) fn preflight(
    repo: &Repo,
    request: &PushRequest,
    remote: &BTreeMap<String, String>,
    cancel: &CancellationToken,
) -> Result<(), PushError> {
    for update in &request.updates {
        if cancel.is_cancelled() {
            return Err(PushError::Cancelled);
        }
        validate_destination(&update.dst)?;
        let old = remote.get(&update.dst).map_or(ZERO, String::as_str);
        if let Some(source) = &update.src {
            let new = repo_ref_exists(repo, source)?;
            ensure_fast_forward(repo, update.force, old, &new)?;
        }
    }
    Ok(())
}

pub(super) fn build_request(
    repo: &Repo,
    request: &PushRequest,
    remote: &BTreeMap<String, String>,
    progress: &dyn PushProgress,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, PushError> {
    let mut commands = Vec::new();
    let mut objects = BTreeSet::new();
    let mut any_upload = false;
    let git = repo.inner().map_err(|_| PushError::SourceNotFound)?;
    for update in &request.updates {
        let old = remote.get(&update.dst).map_or(ZERO, String::as_str);
        let new = match &update.src {
            Some(source) => {
                any_upload = true;
                let id = repo_ref_exists(repo, source)?;
                collect_reachable(&git, id, &mut objects)?;
                id.to_string()
            }
            None => ZERO.to_owned(),
        };
        commands.extend(pkt_line(
            format!(
                "{old} {new} {}\0report-status side-band-64k agent=tether/0.0.4\n",
                update.dst
            )
            .as_bytes(),
        ));
    }
    if commands.is_empty() {
        return Err(PushError::InvalidRequest(String::from("no ref updates")));
    }
    commands.extend(b"0000");
    if !any_upload {
        return Ok(commands);
    }
    progress.update(PushPhase::Packing, 0, Some(objects.len() as u64));
    let pack = make_pack(&git, objects, remote, progress, cancel)?;
    commands.extend(pack);
    Ok(commands)
}

pub(super) fn parse_report(bytes: &[u8], request: &PushRequest) -> Result<PushReport, PushError> {
    if bytes.is_empty() && request.updates.iter().all(|u| u.src.is_none()) {
        return Ok(PushReport {
            updates: request
                .updates
                .iter()
                .map(|u| RefResult {
                    dst: u.dst.clone(),
                    accepted: true,
                })
                .collect(),
        });
    }
    let (text, sideband) = report_parts(bytes)?;
    let text = String::from_utf8_lossy(&text);
    if let Some(line) = text.lines().find(|line| line.starts_with("ng ")) {
        let mut fields = line[3..].splitn(2, ' ');
        let fallback = fields.next().unwrap_or_default().to_owned();
        let reason = sideband
            .into_iter()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        return Err(PushError::Rejected {
            dst: fallback,
            reason: if reason.is_empty() {
                fields.next().unwrap_or("remote rejected").to_owned()
            } else {
                reason
            },
        });
    }
    if !text.contains("unpack ok") {
        return Err(PushError::Protocol);
    }
    let updates = request
        .updates
        .iter()
        .map(|u| RefResult {
            dst: u.dst.clone(),
            accepted: true,
        })
        .collect();
    Ok(PushReport { updates })
}
fn report_parts(bytes: &[u8]) -> Result<(Vec<u8>, Vec<String>), PushError> {
    let mut text = Vec::new();
    let mut sideband = Vec::new();
    for packet in packets(bytes) {
        let packet = packet?;
        if packet.len() > 1 && matches!(packet[0], 1 | 2) {
            decode_sideband(&packet, &mut text, &mut sideband);
        } else {
            text.extend_from_slice(&packet);
        }
    }
    Ok((text, sideband))
}
fn decode_sideband(packet: &[u8], text: &mut Vec<u8>, sideband: &mut Vec<String>) {
    let nested = &packet[1..];
    let Ok(inner) = packets(nested).collect::<Result<Vec<_>, _>>() else {
        append_sideband(packet[0], nested, text, sideband);
        return;
    };
    if inner.is_empty() {
        append_sideband(packet[0], nested, text, sideband);
        return;
    }
    for line in inner {
        append_sideband(packet[0], &line, text, sideband);
    }
}
fn append_sideband(channel: u8, line: &[u8], text: &mut Vec<u8>, sideband: &mut Vec<String>) {
    if channel == 2 {
        sideband.push(String::from_utf8_lossy(line).trim().to_owned());
    }
    text.extend_from_slice(line);
}
fn parse_advertisement(bytes: &[u8]) -> Result<BTreeMap<String, String>, PushError> {
    let mut refs = BTreeMap::new();
    for packet in packets(bytes) {
        let packet = packet?;
        let line = String::from_utf8_lossy(&packet);
        if line.starts_with("# service=") || line.is_empty() {
            continue;
        }
        let line = line.trim_end_matches('\n');
        let (id, name) = line.split_once(' ').ok_or(PushError::Protocol)?;
        let name = name.split('\0').next().unwrap_or(name);
        if !name.ends_with("^{}") {
            refs.insert(name.to_owned(), id.to_owned());
        }
    }
    Ok(refs)
}

fn packets(mut bytes: &[u8]) -> impl Iterator<Item = Result<Vec<u8>, PushError>> + '_ {
    std::iter::from_fn(move || {
        if bytes.is_empty() {
            return None;
        }
        if bytes.len() < 4 {
            bytes = &[];
            return Some(Err(PushError::Protocol));
        }
        let length = usize::from_str_radix(std::str::from_utf8(&bytes[..4]).ok()?, 16).ok()?;
        if length == 0 {
            bytes = &[];
            return Some(Ok(Vec::new()));
        }
        if length < 4 || length > bytes.len() {
            bytes = &[];
            return Some(Err(PushError::Protocol));
        }
        let packet = bytes[4..length].to_vec();
        bytes = &bytes[length..];
        Some(Ok(packet))
    })
}

fn make_pack(
    repo: &gix::Repository,
    mut objects: BTreeSet<gix::hash::ObjectId>,
    remote: &BTreeMap<String, String>,
    progress: &dyn PushProgress,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, PushError> {
    let mut have = BTreeSet::new();
    for id in remote
        .values()
        .filter_map(|id| gix::hash::ObjectId::from_hex(id.as_bytes()).ok())
    {
        let _ignored = collect_reachable(repo, id, &mut have);
    }
    objects.retain(|id| !have.contains(id));
    let mut pack = b"PACK\0\0\0\x02".to_vec();
    pack.extend(
        u32::try_from(objects.len())
            .map_err(|_| PushError::Protocol)?
            .to_be_bytes(),
    );
    for (index, id) in objects.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(PushError::Cancelled);
        }
        let object = repo.find_object(*id).map_err(|_| PushError::Protocol)?;
        let kind = match object.kind {
            gix::object::Kind::Commit => 1,
            gix::object::Kind::Tree => 2,
            gix::object::Kind::Blob => 3,
            gix::object::Kind::Tag => 4,
        };
        pack.extend(pack_header(kind, object.data.len() as u64));
        let mut compressed =
            gix::zlib::stream::deflate::Write::new(Vec::new(), gix::zlib::Compression::DEFAULT);
        compressed
            .write_all(&object.data)
            .map_err(|_| PushError::Protocol)?;
        compressed.flush().map_err(|_| PushError::Protocol)?;
        pack.extend(compressed.into_inner());
        progress.update(
            PushPhase::Packing,
            (index + 1) as u64,
            Some(objects.len() as u64),
        );
    }
    let mut hash = gix::hash::hasher(gix::hash::Kind::Sha1);
    hash.update(&pack);
    let digest = hash.try_finalize().map_err(|_| PushError::Protocol)?;
    pack.extend(digest.as_slice());
    Ok(pack)
}

fn collect_reachable(
    repo: &gix::Repository,
    id: gix::hash::ObjectId,
    out: &mut BTreeSet<gix::hash::ObjectId>,
) -> Result<(), PushError> {
    if !out.insert(id) {
        return Ok(());
    }
    let object = repo.find_object(id).map_err(|_| PushError::Protocol)?;
    match object.kind {
        gix::object::Kind::Commit => {
            let decoded = gix::objs::CommitRef::from_bytes(&object.data, id.kind())
                .map_err(|_| PushError::Protocol)?;
            let tree =
                gix::hash::ObjectId::from_hex(decoded.tree).map_err(|_| PushError::Protocol)?;
            collect_tree(repo, tree, out)?;
            for parent in decoded.parents {
                collect_reachable(
                    repo,
                    gix::hash::ObjectId::from_hex(parent).map_err(|_| PushError::Protocol)?,
                    out,
                )?;
            }
        }
        gix::object::Kind::Tree => collect_tree(repo, id, out)?,
        _ => {}
    }
    Ok(())
}

fn collect_tree(
    repo: &gix::Repository,
    id: gix::hash::ObjectId,
    out: &mut BTreeSet<gix::hash::ObjectId>,
) -> Result<(), PushError> {
    if !out.insert(id) {
        return Ok(());
    }
    let tree_object = repo.find_tree(id).map_err(|_| PushError::Protocol)?;
    let tree = tree_object.decode().map_err(|_| PushError::Protocol)?;
    for entry in tree.entries {
        let child = (*entry.oid).into();
        if entry.mode.is_tree() {
            collect_tree(repo, child, out)?;
        } else {
            out.insert(child);
        }
    }
    Ok(())
}

fn is_ancestor(repo: &Repo, old: &str, new: &str) -> bool {
    let Ok(git) = repo.inner() else {
        return false;
    };
    let Ok(old) = gix::hash::ObjectId::from_hex(old.as_bytes()) else {
        return false;
    };
    let Ok(new) = gix::hash::ObjectId::from_hex(new.as_bytes()) else {
        return false;
    };
    if new == old {
        return true;
    }
    let Ok(mut walk) = git
        .find_commit(new)
        .and_then(|commit| commit.ancestors().all())
    else {
        return false;
    };
    walk.any(|item| item.is_ok_and(|id| id.id() == old))
}

fn rejects_non_fast_forward(
    repo: &Repo,
    force: bool,
    old: &str,
    new: &gix::hash::ObjectId,
) -> bool {
    !force && old != ZERO && !is_ancestor(repo, old, &new.to_string())
}

fn ensure_fast_forward(
    repo: &Repo,
    force: bool,
    old: &str,
    new: &gix::hash::ObjectId,
) -> Result<(), PushError> {
    if rejects_non_fast_forward(repo, force, old, new) {
        Err(PushError::NonFastForward)
    } else {
        Ok(())
    }
}

pub(super) fn repo_ref_exists(repo: &Repo, name: &str) -> Result<gix::hash::ObjectId, PushError> {
    repo.inner()
        .map_err(|_| PushError::SourceNotFound)?
        .rev_parse_single(name)
        .map(gix::Id::detach)
        .map_err(|_| PushError::SourceNotFound)
}
fn validate_destination(dst: &str) -> Result<(), PushError> {
    let Some(branch) = dst.strip_prefix("refs/heads/") else {
        return Err(PushError::InvalidRequest(String::from(
            "remote ref is forbidden",
        )));
    };
    validate_branch_name(branch)
        .map_err(|_| PushError::InvalidRequest(String::from("remote ref is forbidden")))
}
pub(super) fn receive_url(url: &str) -> String {
    url.trim_end_matches('/').to_owned()
}
fn pkt_line(data: &[u8]) -> Vec<u8> {
    let mut out = format!("{:04x}", data.len() + 4).into_bytes();
    out.extend(data);
    out
}
fn pack_header(kind: u8, mut size: u64) -> Vec<u8> {
    let mut out = Vec::new();
    let mut first = kind << 4 | (u8::try_from(size).unwrap_or(u8::MAX) & 0x0f);
    size >>= 4;
    if size > 0 {
        first |= 0x80;
    }
    out.push(first);
    while size > 0 {
        let mut byte = u8::try_from(size).unwrap_or(u8::MAX) & 0x7f;
        size >>= 7;
        if size > 0 {
            byte |= 0x80;
        }
        out.push(byte);
    }
    out
}
