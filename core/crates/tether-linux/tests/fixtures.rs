//! Shared fake PRoot and output fixtures.

#![expect(
    dead_code,
    reason = "shared test fixtures are imported by several integration tests"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use flate2::{Compression, write::GzEncoder};
use tar::{Builder, EntryType, Header};
use tether_linux::{LinuxError, NetworkPolicy, OutputSink, ProotConfig};
use tokio_util::sync::CancellationToken;

pub(crate) fn fake_proot(root: &Path, workspace: &Path) -> PathBuf {
    let path = root.join("proot");
    let script = format!(
        "#!/bin/sh\nrootfs=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  case \"$1\" in\n    -r) rootfs=\"$2\"; shift 2 ;;\n    -w) shift 2 ;;\n    -b) bind=\"$2\"; case \"$bind\" in *:/workspace) cd \"${{bind%:/workspace}}\" ;; esac; shift 2 ;;\n    -0|--link2symlink|--kill-on-exit) shift ;;\n    *) break ;;\n  esac\ndone\ncd \"{}\"\nif [ -d \"$rootfs/usr/bin\" ]; then PATH=\"$rootfs/usr/bin:$PATH\"; export PATH; fi\nexec \"$@\"\n",
        workspace.display()
    );
    fs::write(&path, script).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

pub(crate) fn config(root: &Path, workspace: Option<PathBuf>, proot: PathBuf) -> ProotConfig {
    fs::create_dir_all(root.join("rootfs/alpine")).unwrap();
    fs::create_dir_all(root.join("tmp")).unwrap();
    fs::create_dir_all(root.join("etc")).unwrap();
    fs::write(root.join("etc/resolv.empty"), b"").unwrap();
    fs::write(root.join("etc/resolv.conf"), b"nameserver 127.0.0.1\n").unwrap();
    ProotConfig {
        proot_path: proot,
        loader_path: None,
        linux_root: root.to_owned(),
        workspace,
        network: NetworkPolicy::default(),
    }
}

pub(crate) fn simple_archive() -> Vec<u8> {
    let mut compressed = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut builder = Builder::new(&mut compressed);
        let mut directory = Header::new_gnu();
        directory.set_path("bin").unwrap();
        directory.set_entry_type(EntryType::dir());
        directory.set_size(0);
        directory.set_mode(0o755);
        directory.set_cksum();
        builder.append(&directory, &[][..]).unwrap();
        let bytes = b"busybox";
        let mut file = Header::new_gnu();
        file.set_path("bin/busybox").unwrap();
        file.set_size(bytes.len() as u64);
        file.set_mode(0o755);
        file.set_cksum();
        builder.append(&file, &bytes[..]).unwrap();
        builder.finish().unwrap();
    }
    compressed.finish().unwrap()
}

pub(crate) struct Collected {
    pub(crate) stdout: Vec<Vec<u8>>,
    pub(crate) stderr: Vec<Vec<u8>>,
}

impl Collected {
    pub(crate) fn new() -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }
}

impl OutputSink for Collected {
    fn stdout(&mut self, bytes: &[u8]) {
        self.stdout.push(bytes.to_vec());
    }
    fn stderr(&mut self, bytes: &[u8]) {
        self.stderr.push(bytes.to_vec());
    }
}

pub(crate) struct BytesFetcher {
    pub(crate) bytes: Arc<Vec<u8>>,
    pub(crate) calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl tether_linux::RootfsFetcher for BytesFetcher {
    fn fetch(
        &self,
        _image: &tether_linux::RootfsImage,
        out: &mut dyn std::io::Write,
        cancel: &CancellationToken,
    ) -> Result<(), LinuxError> {
        if cancel.is_cancelled() {
            return Err(LinuxError::Cancelled);
        }
        out.write_all(&self.bytes)
            .map_err(|_| LinuxError::Fetch("test fetch failed".into()))?;
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}
