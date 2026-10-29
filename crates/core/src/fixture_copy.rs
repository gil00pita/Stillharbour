//! Disposable M0 spike, excluded from default/release builds and all IPC.
//! Paths cannot be supplied: every instance owns a new random TempDir.
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem,
};
use rusqlite::Connection;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Intent,
    Created,
    Partial,
    Verified,
    Published,
    Recorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    CopyNotPublished,
    StagedCopyVerified,
    PublishedVerified,
    NeedsAttention,
}

pub struct CopyFixture {
    root: tempfile::TempDir,
}

impl CopyFixture {
    pub fn new() -> Result<Self> {
        let root = tempfile::Builder::new()
            .prefix("stillharbour-m0-")
            .tempdir()?;
        let fixture = Self { root };
        fs::write(fixture.root.path().join("source"), vec![0x6d; 180_000])?;
        fs::write(
            fixture.root.path().join("destination-only"),
            b"unique preserved bytes",
        )?;
        Ok(fixture)
    }

    fn journal(&self) -> Result<Connection> {
        let db = Connection::open(self.root.path().join("journal.sqlite"))?;
        db.execute_batch("PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS operation (id INTEGER PRIMARY KEY CHECK(id=1), state TEXT NOT NULL);")?;
        Ok(db)
    }

    pub fn run_until(&self, stop: Stage) -> Result<()> {
        let db = self.journal()?;
        db.execute("INSERT INTO operation VALUES (1, 'intent')", [])?;
        if stop == Stage::Intent {
            return Ok(());
        }
        let source_path = self.root.path().join("source");
        let mut source = filesystem::open_readonly(&source_path)?;
        let before = source.metadata()?;
        let identity = filesystem::identity(&source)?;
        let staged_path = self.root.path().join("owned-stage");
        let mut staged = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staged_path)?;
        filesystem::sync_directory(self.root.path())?;
        if stop == Stage::Created {
            return Ok(());
        }
        let mut buffer = [0u8; 65536];
        let mut first = true;
        loop {
            let n = source.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            staged.write_all(&buffer[..n])?;
            if first && stop == Stage::Partial {
                staged.sync_all()?;
                return Ok(());
            }
            first = false;
        }
        staged.sync_all()?;
        drop(staged);
        let mut read_back = filesystem::open_readonly(&staged_path)?;
        let after = source.metadata()?;
        if before.len() != after.len()
            || before.modified()? != after.modified()?
            || filesystem::identity(&source)? != identity
            || !filesystem::exact_bytes(&mut source, &mut read_back)?
        {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Fixture source changed or verification failed.",
            ));
        }
        db.execute("UPDATE operation SET state='verified' WHERE id=1", [])?;
        if stop == Stage::Verified {
            return Ok(());
        }
        filesystem::publish_noreplace(&staged_path, &self.root.path().join("published"))?;
        filesystem::sync_directory(self.root.path())?;
        if stop == Stage::Published {
            return Ok(());
        }
        db.execute(
            "UPDATE operation SET state='completed_verified' WHERE id=1",
            [],
        )?;
        Ok(())
    }

    /// Inspection only. No delete, overwrite or blind replay occurs on restart.
    pub fn recover(&self) -> Result<Recovery> {
        let db = self.journal()?;
        let _: String = db.query_row("SELECT state FROM operation WHERE id=1", [], |row| {
            row.get(0)
        })?;
        let source_path = self.root.path().join("source");
        let mut source = filesystem::open_readonly(&source_path)?;
        let published = self.root.path().join("published");
        let staged = self.root.path().join("owned-stage");
        if published.try_exists()? {
            return Ok(
                if filesystem::exact_bytes(
                    &mut source,
                    &mut filesystem::open_readonly(&published)?,
                )? {
                    Recovery::PublishedVerified
                } else {
                    Recovery::NeedsAttention
                },
            );
        }
        if staged.try_exists()?
            && filesystem::exact_bytes(&mut source, &mut filesystem::open_readonly(&staged)?)?
        {
            return Ok(Recovery::StagedCopyVerified);
        }
        Ok(Recovery::CopyNotPublished)
    }

    pub fn source_is_preserved(&self) -> Result<bool> {
        Ok(
            fs::read(self.root.path().join("source"))? == vec![0x6d; 180_000]
                && fs::read(self.root.path().join("destination-only"))?
                    == b"unique preserved bytes",
        )
    }

    pub fn probe_path(&self) -> &Path {
        self.root.path()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    use super::*;
    #[test]
    #[cfg(target_os = "macos")]
    fn interrupted_stages_are_inspected_without_destructive_replay() {
        for stage in [
            Stage::Intent,
            Stage::Created,
            Stage::Partial,
            Stage::Verified,
            Stage::Published,
            Stage::Recorded,
        ] {
            let fixture = CopyFixture::new().unwrap();
            fixture.run_until(stage).unwrap();
            let expected = match stage {
                Stage::Intent | Stage::Created | Stage::Partial => Recovery::CopyNotPublished,
                Stage::Verified => Recovery::StagedCopyVerified,
                Stage::Published | Stage::Recorded => Recovery::PublishedVerified,
            };
            assert_eq!(fixture.recover().unwrap(), expected);
            assert_eq!(fixture.recover().unwrap(), expected);
            assert!(fixture.source_is_preserved().unwrap());
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn unexpected_published_version_is_retained_for_attention() {
        let fixture = CopyFixture::new().unwrap();
        fixture.run_until(Stage::Verified).unwrap();
        fs::write(
            fixture.root.path().join("published"),
            b"another application",
        )
        .unwrap();
        assert_eq!(fixture.recover().unwrap(), Recovery::NeedsAttention);
        assert_eq!(
            fs::read(fixture.root.path().join("published")).unwrap(),
            b"another application"
        );
        assert!(fixture.source_is_preserved().unwrap());
    }
}
