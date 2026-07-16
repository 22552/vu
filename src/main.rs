use std::{fs, io::Write, path::{Path, PathBuf}};

use anyhow::{bail, Context, Result};
use clap::Parser;
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Parser, Debug)]
#[command(version, about = "Download PyPI artifacts without installing or resolving dependencies")]
struct Args {
    /// Requirement in `name` or `name==version` form
    requirement: String,

    /// Directory that receives downloaded artifacts
    #[arg(short = 'd', long, default_value = ".")]
    dest: PathBuf,

    /// Download every artifact published for this release
    #[arg(long)]
    all: bool,

    /// Download source distributions only
    #[arg(long)]
    no_binary: bool,

    /// PyPI-compatible JSON API base URL
    #[arg(long, default_value = "https://pypi.org/pypi")]
    index_url: String,
}

#[derive(Deserialize)]
struct Project { info: Info, urls: Vec<File> }
#[derive(Deserialize)]
struct Info { version: String }
#[derive(Deserialize)]
struct File {
    filename: String,
    url: String,
    packagetype: String,
    yanked: bool,
    digests: Digests,
}
#[derive(Deserialize)]
struct Digests { sha256: String }

fn split_requirement(value: &str) -> Result<(&str, Option<&str>)> {
    let mut parts = value.splitn(2, "==");
    let name = parts.next().unwrap().trim();
    if name.is_empty() || name.contains(['<', '>', '!', '~', '[', ']']) {
        bail!("use an exact requirement: NAME or NAME==VERSION");
    }
    Ok((name, parts.next().map(str::trim).filter(|v| !v.is_empty())))
}

fn is_sdist(file: &File) -> bool { file.packagetype == "sdist" }

fn download(client: &Client, file: &File, dest: &Path) -> Result<()> {
    let path = dest.join(&file.filename);
    let bytes = client.get(&file.url).send()
        .with_context(|| format!("requesting {}", file.filename))?
        .error_for_status()?.bytes()?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != file.digests.sha256 { bail!("checksum mismatch for {}", file.filename); }
    let mut output = fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?;
    output.write_all(&bytes)?;
    println!("Downloaded {}", path.display());
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let (name, requested_version) = split_requirement(&args.requirement)?;
    fs::create_dir_all(&args.dest)?;
    let url = match requested_version {
        Some(version) => format!("{}/{}/{}.json", args.index_url.trim_end_matches('/'), urlencoding::encode(name), urlencoding::encode(version)),
        None => format!("{}/{}.json", args.index_url.trim_end_matches('/'), urlencoding::encode(name)),
    };
    let client = Client::builder().user_agent(concat!("vu/", env!("CARGO_PKG_VERSION"))).build()?;
    let project: Project = client.get(&url).send().with_context(|| format!("looking up {name}"))?.error_for_status()?.json()?;
    let mut files: Vec<&File> = project.urls.iter().filter(|f| !f.yanked && (!args.no_binary || is_sdist(f))).collect();
    if files.is_empty() { bail!("no usable artifacts for {name} {}", project.info.version); }
    if !args.all {
        // An sdist is portable and deterministic; use --all when wheels are wanted too.
        let selected = files.iter().find(|f| is_sdist(f)).copied().unwrap_or(files[0]);
        files = vec![selected];
    }
    for file in files { download(&client, file, &args.dest)?; }
    Ok(())
}
