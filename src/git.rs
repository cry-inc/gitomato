use anyhow::{Context, Result, bail};
use gix::bstr::BStr;
use gix::clone::PrepareFetch;
use gix::create::Kind;
use gix::progress::Discard;
use gix::remote::fetch::Shallow;
use gix::traverse::tree::Recorder;
use std::num::NonZero;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tracing::warn;

pub struct GitCheckout {
    pub hash: String,
    pub files: Vec<GitFile>,
}

pub struct GitFile {
    pub path: String,
    pub hash: String,
    pub data: Vec<u8>,
}

pub fn get_git_files(
    repo: &str,
    refrence: Option<&str>,
    temp_bare_folder: &Path,
    max_bytes: Option<u64>,
) -> Result<GitCheckout> {
    if temp_bare_folder.exists() {
        std::fs::remove_dir_all(temp_bare_folder).context(format!(
            "Failed to delete existing temporary bare folder {}",
            temp_bare_folder.display()
        ))?;
    }
    std::fs::create_dir_all(temp_bare_folder).context(format!(
        "Failed to create temporary bare folder {}",
        temp_bare_folder.display()
    ))?;

    // Prepare shallow clone
    let create_opts = gix::create::Options::default();
    let open_opts = gix::open::Options::default();
    let shallow_clone_depth = NonZero::new(1).context("Depth must be non-zero")?;
    let shallow = Shallow::DepthAtRemote(shallow_clone_depth);
    let partial_name = refrence.map(BStr::new);
    let mut prep = PrepareFetch::new(
        repo.to_string(),
        temp_bare_folder,
        Kind::Bare,
        create_opts,
        open_opts,
    )
    .context("Failed to prepare git fetch")?
    .with_ref_name(partial_name)
    .context("Failed to specify git ref name")?
    .with_shallow(shallow);

    // Execute clone
    let progress = Discard;
    let should_interrupt = AtomicBool::new(false);
    let (repo, _) = prep
        .fetch_only(progress, &should_interrupt)
        .context("Failed to do shallow clone")?;

    // Find commit to check out
    let commit = if let Some(ref_name) = refrence {
        // Search remote references in the cloned bare repository
        let platform = repo
            .references()
            .context("Failed to get references platform from bare clone")?;
        let ref_iter = platform
            .remote_branches()
            .context("Failed to get remote branches")?;
        let mut found_commit = None;
        for reference in ref_iter {
            let Ok(mut reference) = reference else {
                bail!("Cannot get reference");
            };
            let name = reference.name();
            if name.to_string().ends_with(ref_name) {
                found_commit = Some(
                    reference
                        .peel_to_commit()
                        .context("Cannot peel commit from reference")?,
                );
            }
        }
        found_commit.context("Found no commit for reference")?
    } else {
        // Get HEAD commit
        let mut head = repo.head().context("Cannot get head from repository")?;
        head.peel_to_commit()
            .context("Cannot peel commit from head reference")?
    };

    // Walk over files from commit and extract them to page
    let tree = commit.tree().context("Cannot get tree from commit")?;
    let platform = tree.traverse();
    let mut recorder = Recorder::default();
    platform
        .breadthfirst(&mut recorder)
        .context("Failed to start tree traversal")?;

    let mut files = Vec::new();
    let mut bytes_sum = 0;
    for r in recorder.records.iter() {
        if r.mode.is_blob() {
            let blob = repo.find_blob(r.oid).context("Failed to find blob")?;
            let size = blob.data.len() as u64;
            bytes_sum += size;
            if let Some(max) = max_bytes
                && bytes_sum > max
            {
                bail!("Files behind commit are bigger than the limit of {max} bytes");
            }
            files.push(GitFile {
                path: r.filepath.to_string(),
                hash: r.oid.to_string(),
                data: blob.data.clone(),
            });
        }
    }

    // Clear temp folder again
    if let Err(err) = std::fs::remove_dir_all(temp_bare_folder) {
        warn!(
            "Failed to clean temp folder with shallow bare clone in {}: {err}",
            temp_bare_folder.display()
        )
    }

    Ok(GitCheckout {
        hash: commit.id().to_string(),
        files,
    })
}
