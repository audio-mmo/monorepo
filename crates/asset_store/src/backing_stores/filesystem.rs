use std::io::Result;

use camino::Utf8PathBuf;

use super::BackingStoreTrait;

/// A catalog store over the filesystem.
pub(crate) struct FilesystemStore {
    root: Utf8PathBuf,
}

impl FilesystemStore {
    pub(crate) fn new(root: Utf8PathBuf) -> Result<FilesystemStore> {
        if !root.try_exists()? {
            return Err(std::io::ErrorKind::NotFound.into());
        }

        if !root.metadata()?.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{} is not a directory", root),
            ));
        }

        Ok(FilesystemStore { root })
    }

    fn error_if_outside_root(&self, key: &str) -> Result<()> {
        let joined = self.root.join(key);

        if !joined.starts_with(&self.root) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{} is not a subdirectory of {}", joined, self.root),
            ));
        }

        Ok(())
    }
}

fn wrap_walker(root: Utf8PathBuf) -> Result<Box<dyn Iterator<Item = Result<String>>>> {
    let walker = walkdir::WalkDir::new(&root).into_iter()
    .filter_map(move |r| {
        let out = r.and_then(|x| {
            if !x.metadata()?.is_file() {
                return Ok(None);
            }

            let suffix = x.path().strip_prefix(&root).expect("We are walking a directory; all files in that directory should be under the root");
            Ok(Some(                suffix.to_str().expect("Found non-UTF8 catalog data, which should be impossible").to_string()))
        }).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

        match out {
            Ok(None) => None,
            Ok(Some(x))=>Some(Ok(x)),
            Err(e)=>Some(Err(e)),
        }
    });

    Ok(Box::new(walker))
}

impl BackingStoreTrait for FilesystemStore {
    fn open(&self, key: &str) -> Result<Box<dyn std::io::Read>> {
        self.error_if_outside_root(key)?;
        Ok(Box::new(std::fs::File::open(self.root.join(key))?))
    }

    fn iter_all(&self) -> Result<super::DirIter> {
        wrap_walker(self.root.clone())
    }

    fn iter_subdir(&self, prefix: &str) -> Result<super::DirIter> {
        let root = self.root.join(prefix);
        if !root.try_exists()? {
            return Err(std::io::ErrorKind::NotFound.into());
        }

        wrap_walker(root)
    }
}
