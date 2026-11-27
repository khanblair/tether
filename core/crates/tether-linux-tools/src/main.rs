//! Prepare and verify signed Linux root filesystem manifests.

use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use tether_linux::{RootfsImage, RootfsManifest, verify_rootfs_manifest};

const BASE_PACKAGES: [&str; 7] = [
    "bash",
    "git",
    "nodejs",
    "npm",
    "python3",
    "py3-pip",
    "build-base",
];
const MAX_EXTRACTED_BYTES: u64 = 1024 * 1024 * 1024;

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let result = run(&arguments);
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run(arguments: &[String]) -> Result<(), String> {
    let (command, rest) = arguments
        .split_first()
        .ok_or_else(|| "a subcommand is required".to_owned())?;
    let options = parse_options(rest)?;
    match command.as_str() {
        "keygen" => keygen(&options),
        "sign" => sign(&options),
        "verify" => verify(&options),
        _ => Err("unknown subcommand".to_owned()),
    }
}

fn parse_options(arguments: &[String]) -> Result<HashMap<String, String>, String> {
    if !arguments.len().is_multiple_of(2) {
        return Err("options must be name and value pairs".to_owned());
    }
    arguments
        .chunks(2)
        .try_fold(HashMap::new(), |mut options, pair| {
            let name = pair.first().ok_or_else(|| "invalid option".to_owned())?;
            let value = pair.get(1).ok_or_else(|| "invalid option".to_owned())?;
            if !name.starts_with("--") || options.insert(name.clone(), value.clone()).is_some() {
                return Err("invalid or repeated option".to_owned());
            }
            Ok(options)
        })
}

fn required(options: &HashMap<String, String>, name: &str) -> Result<String, String> {
    options
        .get(name)
        .cloned()
        .ok_or_else(|| format!("missing {name}"))
}

fn keygen(options: &HashMap<String, String>) -> Result<(), String> {
    let path = PathBuf::from(required(options, "--out")?);
    let mut secret = [0_u8; 32];
    File::open("/dev/urandom")
        .map_err(|_| "could not open the operating system random source".to_owned())?
        .read_exact(&mut secret)
        .map_err(|_| "could not read the operating system random source".to_owned())?;
    let signing_key = SigningKey::from_bytes(&secret);
    let mut file = create_private_key(&path)?;
    file.write_all(hex(&secret).as_bytes())
        .map_err(|_| "could not write the private key".to_owned())?;
    file.flush()
        .map_err(|_| "could not finish writing the private key".to_owned())?;
    println!(
        "public key: {}",
        hex(signing_key.verifying_key().as_bytes())
    );
    println!("private key path: {}", path.display());
    Ok(())
}

#[cfg(unix)]
fn create_private_key(path: &Path) -> Result<File, String> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| "could not create the private key".to_owned())?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|_| "could not set private key permissions".to_owned())?;
    Ok(file)
}

#[cfg(not(unix))]
fn create_private_key(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| "could not create the private key".to_owned())
}

fn sign(options: &HashMap<String, String>) -> Result<(), String> {
    let key_path = PathBuf::from(required(options, "--key")?);
    let rootfs_path = PathBuf::from(required(options, "--rootfs")?);
    let url = required(options, "--url")?;
    let alpine_version = required(options, "--alpine-version")?;
    let out_dir = PathBuf::from(required(options, "--out-dir")?);
    if !url.starts_with("https://") {
        return Err("rootfs URL must start with https://".to_owned());
    }
    let signing_key = read_signing_key(&key_path)?;
    let (sha256, size_bytes) = archive_digest(&rootfs_path)?;
    let manifest = RootfsManifest {
        format: 1,
        alpine_version,
        images: vec![RootfsImage {
            arch: "aarch64".to_owned(),
            url,
            sha256,
            size_bytes,
            max_extracted_bytes: MAX_EXTRACTED_BYTES,
        }],
        base_packages: BASE_PACKAGES
            .iter()
            .map(|package| (*package).to_owned())
            .collect(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|_| "could not serialize the manifest".to_owned())?;
    fs::create_dir_all(&out_dir).map_err(|_| "could not create the output directory".to_owned())?;
    let manifest_path = out_dir.join("manifest.json");
    let signature_path = out_dir.join("manifest.json.sig");
    fs::write(&manifest_path, &manifest_bytes)
        .map_err(|_| "could not write the manifest".to_owned())?;
    let signature = signing_key.sign(&manifest_bytes).to_bytes();
    fs::write(&signature_path, signature)
        .map_err(|_| "could not write the manifest signature".to_owned())?;
    verify_rootfs_manifest(
        &manifest_bytes,
        &signature,
        signing_key.verifying_key().as_bytes(),
    )
    .map_err(|_| "the generated manifest did not verify".to_owned())?;
    println!(
        "public key: {}",
        hex(signing_key.verifying_key().as_bytes())
    );
    println!("manifest: {}", manifest_path.display());
    println!("signature: {}", signature_path.display());
    Ok(())
}

fn verify(options: &HashMap<String, String>) -> Result<(), String> {
    let public_key = decode_key(&required(options, "--public-key")?)?;
    let manifest_path = PathBuf::from(required(options, "--manifest")?);
    let signature_path = PathBuf::from(required(options, "--sig")?);
    let manifest =
        fs::read(&manifest_path).map_err(|_| "could not read the manifest".to_owned())?;
    let signature =
        fs::read(&signature_path).map_err(|_| "could not read the signature".to_owned())?;
    let key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| "public key must be 64 hexadecimal characters".to_owned())?;
    if verify_rootfs_manifest(&manifest, &signature, &key).is_ok() {
        println!("valid");
        return Ok(());
    }
    println!("invalid");
    std::process::exit(1);
}

fn read_signing_key(path: &Path) -> Result<SigningKey, String> {
    let text = fs::read_to_string(path).map_err(|_| "could not read the private key".to_owned())?;
    let bytes = decode_key(text.trim())?;
    let secret: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "private key must be 64 hexadecimal characters".to_owned())?;
    Ok(SigningKey::from_bytes(&secret))
}

fn decode_key(value: &str) -> Result<Vec<u8>, String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("key must be 64 hexadecimal characters".to_owned());
    }
    value
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let high = pair
                .first()
                .and_then(|byte| hex_value(*byte))
                .ok_or_else(|| "key must be 64 hexadecimal characters".to_owned())?;
            let low = pair
                .get(1)
                .and_then(|byte| hex_value(*byte))
                .ok_or_else(|| "key must be 64 hexadecimal characters".to_owned())?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn archive_digest(path: &Path) -> Result<(String, u64), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| "could not inspect the rootfs archive".to_owned())?;
    if !metadata.file_type().is_file() {
        return Err("rootfs archive must be a regular file".to_owned());
    }
    let mut file = File::open(path).map_err(|_| "could not open the rootfs archive".to_owned())?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| "could not read the rootfs archive".to_owned())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok((hex(&hasher.finalize()), metadata.len()))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                HEX[usize::from(byte >> 4)] as char,
                HEX[usize::from(byte & 15)] as char,
            ]
        })
        .collect()
}
