//! Faculties for interacting with the `openwdl/wdl` repository.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use bon::Builder;
use git2::FetchOptions;
use tracing::info;

/// The default URL for the `openwdl/wdl` repository.
const REPOSITORY_URL: &str = "https://github.com/openwdl/wdl.git";

/// The WDL specification repository.
#[derive(Builder)]
#[builder(builder_type = Builder)]
pub struct Repository {
    /// The local directory.
    ///
    /// An empty local directory signifies that a temporary directory should be created
    /// upon checkout.
    // NOTE: this is not created as a default with the `bon` builder because we don't
    // want to create a new temporary directory with every test.
    local_dir: Option<PathBuf>,

    /// The branch to check out.
    #[builder(into)]
    branch: String,

    /// The remote url.
    #[builder(default = REPOSITORY_URL.to_owned())]
    url: String,
}

impl Repository {
    /// Checks out the repository and returns a [`git2::Repository`].
    pub fn checkout(self) -> Result<(git2::Repository, PathBuf)> {
        let path = self.local_dir.unwrap_or_else(|| {
            // SAFETY: on all the platforms we support, we expect a temporary
            // directory to be able to be created.
            let path = tempfile::tempdir()
                .expect("temporary directory to create")
                .into_path()
                .join("wdl");

            info!(
                "created temporary directory for repository at `{}`",
                path.display()
            );

            path
        });

        if path.exists() {
            // If the directory already exists, that directory is assumed to be
            // the git repository checked out on a different run.
            info!("using existing git repository");
            return git2::Repository::open(&path)
                .map(|repo| (repo, path))
                .map_err(Into::into);
        }

        info!(
            "creating new git repository with branch `{branch}`",
            branch = self.branch
        );
        let mut fetch_options = FetchOptions::new();
        fetch_options.depth(1);

        git2::build::RepoBuilder::new()
            .branch(&self.branch)
            .fetch_options(fetch_options)
            .clone(&self.url, &path)
            .map(|repo| (repo, path))
            .map_err(Into::into)
    }

    /// Gets a reference to the local directory.
    pub fn local_dir(&self) -> Option<&Path> {
        self.local_dir.as_deref()
    }

    /// Gets a reference to the URL.
    pub fn url(&self) -> &str {
        &self.url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_url() {
        let repo = Repository::builder().branch("main").build();

        assert!(repo.local_dir.is_none());
        assert_eq!(repo.url(), REPOSITORY_URL);
    }
}
