use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tether_engine::updates::{Asset, Release, UpdateCheck, UpdateError};
use tokio_util::sync::CancellationToken;

use super::*;

struct FakeChecker {
    check_result: Mutex<Result<UpdateCheck, UpdateError>>,
    download_result: Mutex<Result<PathBuf, UpdateError>>,
}

impl Checker for FakeChecker {
    fn check(&self, _current_version: &str) -> Result<UpdateCheck, UpdateError> {
        self.check_result.lock().unwrap().clone()
    }

    fn download(&self, request: DownloadRequest<'_>) -> Result<PathBuf, UpdateError> {
        (request.progress)(0, request.release.apk.size);
        (request.progress)(request.release.apk.size, request.release.apk.size);
        self.download_result.lock().unwrap().clone()
    }
}

struct Progress(Mutex<Vec<(u64, u64)>>);

impl DownloadProgress for Progress {
    fn on_progress(&self, downloaded: u64, total: u64) {
        self.0.lock().unwrap().push((downloaded, total));
    }
}

fn release() -> Release {
    Release {
        version: "0.0.3".into(),
        tag: "v0.0.3".into(),
        notes: "notes".into(),
        published_at: "now".into(),
        apk: Asset {
            name: "tether-full-v0.0.3.apk".into(),
            url: "https://github.com/x.apk".into(),
            size: 4,
        },
        sums: Asset {
            name: "SHA256SUMS".into(),
            url: "https://github.com/x.sums".into(),
            size: 64,
        },
    }
}

fn updater(
    check: Result<UpdateCheck, UpdateError>,
    download: Result<PathBuf, UpdateError>,
) -> Updater {
    Updater::with_checker(
        PathBuf::from("downloads"),
        Arc::new(FakeChecker {
            check_result: Mutex::new(check),
            download_result: Mutex::new(download),
        }),
    )
}

#[test]
fn check_caches_available_release_and_download_uses_its_version() {
    let release = release();
    let updater = updater(
        Ok(UpdateCheck::Available(release.clone())),
        Ok(PathBuf::from("downloads/app.apk")),
    );
    let view = updater.check("0.0.2".into()).unwrap();
    assert!(matches!(view, UpdateCheckView::Available { release } if release.version == "0.0.3"));
    let progress = Arc::new(Progress(Mutex::new(Vec::new())));
    let path = updater.download("0.0.3".into(), progress.clone()).unwrap();
    assert!(path.ends_with("downloads/app.apk"));
    assert_eq!(*progress.0.lock().unwrap(), vec![(0, 4), (4, 4)]);
    assert_eq!(
        updater.download("0.0.4".into(), progress),
        Err(UpdaterError::NoRelease)
    );
}

#[test]
fn update_errors_map_one_to_one_and_cancel_is_harmless() {
    for (engine, ffi) in [
        (UpdateError::Network, UpdaterError::Network),
        (UpdateError::Unavailable, UpdaterError::Unavailable),
        (UpdateError::Invalid, UpdaterError::Invalid),
        (UpdateError::Checksum, UpdaterError::Checksum),
        (UpdateError::TooLarge, UpdaterError::TooLarge),
        (UpdateError::Storage, UpdaterError::Storage),
        (UpdateError::Cancelled, UpdaterError::Cancelled),
    ] {
        assert_eq!(UpdaterError::from(engine), ffi);
    }
    let updater = updater(Err(UpdateError::Network), Ok(PathBuf::from("app.apk")));
    updater.cancel();
    assert_eq!(updater.check("0.0.2".into()), Err(UpdaterError::Network));
}

#[test]
fn cancel_does_not_poison_a_later_download() {
    let release = release();
    let updater = updater(
        Ok(UpdateCheck::Available(release)),
        Ok(PathBuf::from("app.apk")),
    );
    updater.check("0.0.2".into()).unwrap();
    updater.cancel();
    let progress = Arc::new(Progress(Mutex::new(Vec::new())));
    assert!(updater.download("0.0.3".into(), progress).is_ok());
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
}
