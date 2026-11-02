use super::*;

pub(super) fn companions(
    rows: &mut [ComparisonEntry],
    observations: &BTreeMap<String, (PathBuf, Option<Snapshot>, Option<Snapshot>)>,
) {
    let mut groups = BTreeMap::<(PathBuf, String), Vec<usize>>::new();
    for (index, row) in rows.iter().enumerate() {
        let Some((path, _, _)) = observations.get(&row.id) else {
            continue;
        };
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let mut stem = name.to_lowercase();
        if let Some((base, extension)) = stem.rsplit_once('.') {
            let sidecar = matches!(extension, "xmp" | "acr" | "aae" | "json");
            stem = base.to_owned();
            if sidecar && let Some((base, _)) = stem.rsplit_once('.') {
                stem = base.to_owned();
            }
        }
        groups
            .entry((path.parent().unwrap_or(Path::new("")).to_owned(), stem))
            .or_default()
            .push(index);
    }
    let mut text_budget = 2 * 1024 * 1024usize;
    for group in groups.values().filter(|g| g.len() > 1) {
        let displayed: Vec<_> = group
            .iter()
            .take(33)
            .map(|i| (*i, rows[*i].display_path.clone()))
            .collect();
        for index in group {
            rows[*index].companions = displayed
                .iter()
                .filter(|(i, _)| i != index)
                .take(32)
                .filter_map(|(_, path)| {
                    if path.len() > text_budget {
                        None
                    } else {
                        text_budget -= path.len();
                        Some(path.clone())
                    }
                })
                .collect();
        }
    }
}

impl Library {
    pub fn backup_comparison_preview(
        &self,
        run_id: String,
        entry_id: String,
        side: ComparisonSide,
    ) -> Result<ComparisonPreview> {
        let _gate = self.comparisons.preview_gate.try_lock().map_err(|_| {
            error(
                ErrorCode::Busy,
                "One comparison preview is already running.",
            )
        })?;
        let (binding, path, snapshot, cancel, source, root_id) = {
            let state = self.comparisons.review.lock().map_err(|_| unavailable())?;
            let review = state
                .as_ref()
                .filter(|r| {
                    r.run.id == run_id
                        && matches!(
                            r.run.state,
                            ComparisonState::NeedsReview | ComparisonState::Incomplete
                        )
                })
                .ok_or_else(unavailable)?;
            let row = review
                .rows
                .iter()
                .find(|r| r.id == entry_id)
                .ok_or_else(unavailable)?;
            if row.category == ComparisonCategory::Blocked {
                return Err(error(
                    ErrorCode::Unsupported,
                    "This entry is blocked. Retain both trees and review its text evidence; a preview cannot resolve the conflict.",
                ));
            }
            let (path, a, b) = review.observations.get(&row.id).ok_or_else(unavailable)?;
            let (binding, snapshot) = match side {
                ComparisonSide::Source => (review.source.as_ref(), a.as_ref()),
                ComparisonSide::Destination => (review.destination.as_ref(), b.as_ref()),
                ComparisonSide::Comparison => return Err(unavailable()),
            };
            (
                binding.ok_or_else(||error(ErrorCode::Unsupported,"No file was observed on this side of the comparison; there is no image to preview."))?.clone(),
                path.clone(),
                snapshot.ok_or_else(||error(ErrorCode::Unsupported,"No regular file was observed on this side of the comparison; retain the text observations."))?.clone(),
                review.cancel.clone(),
                review.source.as_ref().ok_or_else(unavailable)?.clone(),
                review.run.source_root_id.clone(),
            )
        };
        self.validate_comparison_preview_root(&source, &root_id)?;
        let _lease = Registry::shared().try_acquire(vec![Scope::new(
            DirectoryChain::open(&binding.path.canonicalize()?)?,
            Access::Read,
        )])?;
        canceled(&cancel)?;
        if snapshot.bytes > crate::media::MAX_INPUT_BYTES {
            return Err(error(
                ErrorCode::Unsupported,
                "Preview exceeds the 32 MiB input budget; use text observations.",
            ));
        }
        let mut file = binding.file(&path)?;
        snapshot.validate(&file)?;
        let mut bytes = Vec::with_capacity(snapshot.bytes as usize);
        let mut buffer = [0; CHUNK_BYTES];
        let mut count = 0;
        loop {
            let n = fill(&mut file, &mut buffer, &cancel, &mut count)?;
            if n == 0 {
                break;
            }
            if bytes.len() + n > crate::media::MAX_INPUT_BYTES as usize {
                return Err(changed());
            }
            bytes.extend_from_slice(&buffer[..n]);
        }
        snapshot.validate(&file)?;
        snapshot.validate(&binding.file(&path)?)?;
        let format = crate::media::PreviewFormat::from_extension(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default(),
        );
        let report =
            tokio::runtime::Handle::current().block_on(super::super::workers::thumbnail(
                &self.options.workers,
                bytes,
                512,
                format,
                cancel.clone(),
            ))?;
        snapshot.validate(&binding.file(&path)?)?;
        canceled(&cancel)?;
        self.validate_comparison_preview_root(&source, &root_id)?;
        if self.backup_comparison(run_id.clone())?.id != run_id {
            return Err(unavailable());
        }
        Ok(ComparisonPreview {
            run_id,
            entry_id,
            side,
            png: report.png,
            message: format!(
                "Read-only 512 px preview; {}. Preview is not integrity, fidelity or recovery evidence.",
                report.color_policy
            ),
        })
    }
    pub(super) fn validate_comparison_preview_root(
        &self,
        source: &Binding,
        root_id: &str,
    ) -> Result<()> {
        let root = store::root(&self.catalog, root_id.into())?;
        if root.path != source.path
            || !same_directory(&root.identity, &source.identity)
            || root.volume != source.volume
            || root.view.availability != "online"
        {
            return Err(changed());
        }
        source.open()?;
        Ok(())
    }
}
