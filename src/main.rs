use lazy_static::lazy_static;
use libnar::{
    de::{Entries, EntryKind},
    Archive,
};
use regex::Regex;
use reqwest;
use std::env;
use std::ffi::OsString;
use std::fs::{self};
use std::io::{self, Error, ErrorKind};
use std::path::Path;
use xz::read::XzDecoder;

#[derive(Debug)]
enum MyError {
    E1(),
    E2(reqwest::Error),
    E3(std::io::Error),
    E4(OsString),
}

impl From<reqwest::Error> for MyError {
    fn from(err: reqwest::Error) -> MyError {
        MyError::E2(err)
    }
}

impl From<std::io::Error> for MyError {
    fn from(err: std::io::Error) -> MyError {
        MyError::E3(err)
    }
}

impl From<OsString> for MyError {
    fn from(err: OsString) -> MyError {
        MyError::E4(err)
    }
}

fn extract_hash(input: &str) -> Option<&str> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r".*/?(?P<hash>[a-zA-Z0-9]{32})-?.*").unwrap();
    }
    RE.captures(input)
        .and_then(|cap| cap.name("hash").map(|hash| hash.as_str()))
}

fn create_dir(dst: &Path) -> io::Result<()> {
    fs::create_dir_all(&dst).or_else(|err| {
        if err.kind() == ErrorKind::AlreadyExists {
            let prev = fs::metadata(&dst);
            if prev.map(|m| m.is_dir()).unwrap_or(false) {
                return Ok(());
            }
        }
        Err(Error::new(
            err.kind(),
            format!("{} when creating dir {}", err, dst.display()),
        ))
    })
}

fn recurse<T: std::io::Read>(entries: Entries<T>, dst: &Path) -> Result<(), MyError> {
    create_dir(dst)?;
    for entry in entries {
        let mut entry = entry?;
        if entry.is_dir() {
            println!("Creating {:?}", dst.join(entry.name()));
            create_dir(&dst.join(entry.name()))?;
        } else if entry.is_file() || entry.is_executable() {
            println!(
                "Extracting file {:?} to {:?}",
                entry.name(),
                dst.join(entry.name())
            );
            entry.unpack_in(dst)?
        } else if entry.is_symlink() {
            println!("Symlink encountered. Recursing.");
            if let EntryKind::Symlink { target } = entry.kind {
                println!("{:?}", target);
                download(&target.into_os_string().into_string()?)?;
            }
        } else {
            println!("Unknown file type ! Skipping.");
        }
    }
    Ok(())
}

fn get_archive_url(hash: &str) -> Result<String, MyError> {
    let binary_caches = [
        "https://cache.nixos.org/",
        "https://sisyphe.cachix.org/",
        "https://bincache.grunblatt.org/",
    ];
    for cache in binary_caches.iter() {
        let url = format!("{}{}.narinfo", cache.to_owned(), hash);
        let resp = reqwest::blocking::get(url)?;
        match resp.status() {
            reqwest::StatusCode::OK => {
                return Ok(cache.to_string()
                    + &resp.text()?.split("\n").collect::<Vec<&str>>()[1]
                        .to_string()
                        .split(": ")
                        .collect::<Vec<&str>>()[1]
                        .to_string());
            }
            _ => {}
        }
    }
    return Err(MyError::E1());
}

fn download(path_or_hash: &str) -> Result<(), MyError> {
    let hash = extract_hash(path_or_hash).unwrap();
    /* Search hash narinfo in binary caches */
    let url = get_archive_url(hash)?;
    println!("{}", url);
    /* Get the archive if it has been found */
    let resp = reqwest::blocking::get(url)?;
    match resp.status() {
        reqwest::StatusCode::OK => {
            /* Decompress the archive using xz */
            let decompressor = XzDecoder::new(resp);
            let mut nar = Archive::new(decompressor);
            let entries = nar.entries().unwrap();
            let dirname = format!("./result/{}", hash);
            let path = Path::new(&dirname);
            recurse(entries, path)
        }
        _ => Err(MyError::E1()),
    }
}

fn main() -> Result<(), MyError> {
    let hash = env::args()
        .nth(1)
        .expect("Expected path name or hash of a nar archive");
    download(&hash)?;
    Ok(())
}
