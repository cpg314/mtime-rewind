use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use log::*;
use serde::{Deserialize, Serialize};
use sha2::Digest;

/// Rewind the mtime of files whose mtime advanced since the last execution without a content change.
#[derive(Parser)]
struct Flags {
    root: PathBuf,
    /// Do not edit only mtime, only list the changes that would be made.
    #[clap(long)]
    dry: bool,
}
#[derive(Serialize, Deserialize, Debug)]
struct Entry {
    hash: Vec<u8>,
    mtime: std::time::SystemTime,
}

impl Entry {
    fn from_file(filename: &Path) -> anyhow::Result<Self> {
        let mut hasher = sha2::Sha256::new();
        let file = std::fs::File::open(filename)?;
        let mut file = std::io::BufReader::new(file);
        std::io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();

        let meta = std::fs::metadata(filename)?;
        Ok(Self {
            hash: hash.to_vec(),
            mtime: meta.modified()?,
        })
    }
}
#[derive(Serialize, Deserialize)]
struct Data {
    data: HashMap<PathBuf, Entry>,
    root: PathBuf,
}
impl Data {
    fn compute(root: &Path) -> anyhow::Result<Self> {
        info!("Computing hashes...");
        let files = walkdir::WalkDir::new(root)
            .min_depth(1)
            .into_iter()
            // Skip hidden entries and cache folders (e.g. cargo's target fodlers)
            .filter_entry(|e| {
                !e.path().join("CACHEDIR.TAG").exists()
                    && !e
                        .path()
                        .file_name()
                        .and_then(|f| f.to_str())
                        .map_or(false, |f| f.starts_with('.'))
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.metadata().map_or(false, |e| e.is_file()));

        // Compute current hashes
        let mut data = HashMap::default();
        for entry in files {
            data.insert(entry.path().into(), Entry::from_file(entry.path())?);
        }
        info!("Computed hashes for {} files", data.len());
        Ok(Self {
            data,
            root: root.into(),
        })
    }
    fn hashes_file(root: &Path) -> PathBuf {
        root.join(".hashprint")
    }
    fn load_cached(root: &Path) -> anyhow::Result<Self> {
        info!("Loading cached state...");
        let cached =
            std::fs::read(Self::hashes_file(root)).context("Could not open hash file.")?;
        let cached: Self = bincode::deserialize(&cached)?;
        anyhow::ensure!(
            cached.root == root,
            "Mismatching roots found: {:?} vs {:?}",
            cached.root,
            root
        );
        info!("Loaded hashes for {:?} files", cached.data.len());
        Ok(cached)
    }
    fn save(&self) -> anyhow::Result<()> {
        let output = Self::hashes_file(&self.root);
        std::fs::write(&output, bincode::serialize(&self)?)?;
        info!("Wrote {:?}", output);
        Ok(())
    }
}
fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Flags::parse();

    let live = Data::compute(&args.root)?;

    if !Data::hashes_file(&args.root).exists() {
        info!("Writing hashes for the first time...");
        live.save()?;
    } else {
        info!("Restoring modification times for unchanged files...");
        let stored = Data::load_cached(&args.root)?;

        let mut edited = HashMap::<PathBuf, Entry>::default();
        for (f, stored) in stored.data {
            if let Some(live) = live.data.get(&f) {
                debug!("{:?}: {:?} (live) vs {:?} (stored)", f, live, stored);
                // Find files whose contents haven't changed, yet the mtime is set to later than
                // on the previous run
                if live.mtime > stored.mtime {
                    if live.hash != stored.hash {
                        // Legitimate mtime increase
                        info!("{:?} was actually modified", f);
                    } else {
                        info!(
                            "Rewinding {:?} from {:?} to {:?} as its contents did not change",
                            f, live.mtime, stored.mtime
                        );
                        if args.dry {
                            warn!("Dry mode, not applying changes");
                        } else {
                            filetime::set_file_mtime(
                                &f,
                                filetime::FileTime::from_system_time(stored.mtime),
                            )?;
                            edited.insert(f, stored);
                        }
                    }
                }
            }
        }

        info!("{} files rewinded", edited.len());
        // Apply the new state before saving
        let mut live = live;
        live.data.extend(edited);
        if !args.dry {
            info!("Saving the new state...");
            live.save()?;
        }
    }
    info!("Done");
    Ok(())
}
