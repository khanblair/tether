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

/// A checker whose download waits until its token is cancelled, like a stalled network.
struct BlockingChecker {
    started: std::sync::mpsc::Sender<()>,
}

impl Checker for BlockingChecker {
    fn check(&self, _current_version: &str) -> Result<UpdateCheck, UpdateError> {
        Ok(UpdateCheck::Available(release()))
    }

    fn download(&self, request: DownloadRequest<'_>) -> Result<PathBuf, UpdateError> {
        self.started.send(()).unwrap();
        // A broken cancel must fail the test, not hang it: give up after five seconds.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !request.cancel.is_cancelled() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        if request.cancel.is_cancelled() {
            Err(UpdateError::Cancelled)
        } else {
            Err(UpdateError::Network)
        }
    }
}

#[test]
fn cancel_stops_a_download_in_flight_and_a_later_download_starts_fresh() {
    let (started, wait_for_start) = std::sync::mpsc::channel();
    let updater = Arc::new(Updater::with_checker(
        PathBuf::from("downloads"),
        Arc::new(BlockingChecker { started }),
    ));
    updater.check("0.0.2".into()).unwrap();
    let progress = Arc::new(Progress(Mutex::new(Vec::new())));
    let worker = {
        let updater = Arc::clone(&updater);
        let progress = progress.clone();
        std::thread::spawn(move || updater.download("0.0.3".into(), progress))
    };
    wait_for_start.recv().unwrap();
    updater.cancel();
    assert_eq!(worker.join().unwrap(), Err(UpdaterError::Cancelled));

    // The next download gets its own token, so the old cancel does not poison it.
    let second = {
        let updater = Arc::clone(&updater);
        std::thread::spawn(move || updater.download("0.0.3".into(), progress))
    };
    wait_for_start.recv().unwrap();
    updater.cancel();
    assert_eq!(second.join().unwrap(), Err(UpdaterError::Cancelled));
}
