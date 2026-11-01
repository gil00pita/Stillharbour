//! Host-runner-only probe. Scope comes from the owned virtual-volume harness,
//! never Tauri, a renderer, a user library or arbitrary command-line paths.
use super::*;
use crate::{library::native, operations::*};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Scope {
    version: u32,
    run_id: String,
    image: PathBuf,
    destination_parent: PathBuf,
}
fn scope() -> Result<Scope> {
    let id = std::env::var("STILLHARBOUR_CROSS_VOLUME_RUN")
        .map_err(|_| failed("Use the owned cross-volume harness; no volume scope was supplied."))?;
    crate::operations::validate_id(&id)?;
    checked_scope(&id, &std::env::current_dir()?)
}
fn checked_scope(id: &str, workspace: &Path) -> Result<Scope> {
    crate::operations::validate_id(id)?;
    let workspace = workspace.canonicalize()?;
    let directory = workspace
        .join(".context")
        .join("cross-volume")
        .join(id)
        .canonicalize()?;
    if !directory.starts_with(workspace.join(".context").join("cross-volume")) {
        return Err(failed("Cross-volume evidence escaped the task workspace."));
    }
    let text = std::fs::read(directory.join("scope.json"))?;
    if text.len() > 16384 {
        return Err(failed("Cross-volume scope exceeds its bound."));
    }
    let scope: Scope = serde_json::from_slice(&text)
        .map_err(|error| failed("Invalid virtual-volume scope.").caused_by(error))?;
    let image = scope.image.canonicalize()?;
    if scope.version != 1
        || scope.run_id != id
        || image.parent() != Some(directory.as_path())
        || !image.is_file()
        || !matches!(
            image.file_name().and_then(|name| name.to_str()),
            Some("destination.vhdx" | "destination.sparseimage")
        )
    {
        return Err(failed(
            "The image does not belong to this cross-volume test run.",
        ));
    }
    let parent = scope.destination_parent.canonicalize()?;
    if parent.file_name().and_then(|name| name.to_str())
        != Some(format!("stillharbour-cross-volume-{id}").as_str())
    {
        return Err(failed(
            "The destination is not the constructor-owned fixture parent.",
        ));
    }
    let marker = std::fs::read(parent.join(".stillharbour-fixture-owner"))?;
    if marker != id.as_bytes() {
        return Err(failed("The destination fixture ownership marker changed."));
    }
    let chain = crate::filesystem::DirectoryChain::open(&parent)?;
    let backing = native::open_root(&directory, None)?;
    if Object::of(chain.leaf())?.volume == Object::of(&backing)?.volume
        || native::volume_token(chain.leaf())? == native::volume_token(&backing)?
    {
        return Err(failed(
            "The fixture destination is not a different native volume.",
        ));
    }
    Ok(Scope {
        destination_parent: parent,
        image,
        ..scope
    })
}
fn wait(load: impl Fn() -> Result<CopyWorkflowJob>) -> Result<CopyWorkflowJob> {
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        let job = load()?;
        if !matches!(
            job.state,
            CopyJobState::Queued
                | CopyJobState::Running
                | CopyJobState::PauseRequested
                | CopyJobState::CancelRequested
        ) {
            return Ok(job);
        }
        if Instant::now() >= until {
            return Err(failed(
                "Cross-volume fixture worker did not stop within its bound.",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
pub(in crate::executor) fn run() -> Result<serde_json::Value> {
    let scope = scope()?;
    let fixture = Fixture::with_media_at(
        true,
        generated_photo_bytes()?,
        Some(&scope.destination_parent),
    )?;
    if fixture.plan.source.volume == fixture.plan.destination.volume
        || fixture.plan.source_volume == fixture.plan.destination_volume
    {
        return Err(failed(
            "Cross-volume copy requires two actual native volume identities.",
        ));
    }
    let selected = native::open_root(&fixture.destination, None)?;
    let identity = crate::filesystem::identity(&selected)?;
    let volume = native::volume_token(&selected)?;
    drop(selected);
    let selection = uuid::Uuid::new_v4().to_string();
    let mut review = fixture.library.organization_copy_review_create(
        fixture.owner.clone(),
        fixture.manifest.clone(),
        selection.clone(),
        fixture.destination.clone(),
        (identity.clone(), volume.clone()),
    )?;
    let mut jobs = Vec::new();
    for action in [CopyAction::PrepareNames, CopyAction::CopyFiles] {
        let approval = fixture.library.organization_copy_approve(
            CopyApprovalInput {
                review_id: review.id.clone(),
                digest: review.digest.clone(),
                action,
                fidelity_limits_acknowledged: true,
            },
            selection.clone(),
            fixture.destination.clone(),
            (identity.clone(), volume.clone()),
        )?;
        let queued = fixture.library.organization_copy_start(
            CopyStartInput {
                review_id: review.id.clone(),
                digest: review.digest.clone(),
                approval_id: approval.id,
            },
            selection.clone(),
            fixture.destination.clone(),
            (identity.clone(), volume.clone()),
        )?;
        let stopped = wait(|| fixture.library.organization_copy_job(queued.id.clone()))?;
        if stopped.state
            != if action == CopyAction::PrepareNames {
                CopyJobState::Prepared
            } else {
                CopyJobState::CompletedVerified
            }
        {
            return Err(failed(
                "Cross-volume application job did not complete its exact verified action.",
            ));
        }
        jobs.push(stopped);
        if action == CopyAction::PrepareNames {
            review = fixture.library.organization_copy_review(
                review.id,
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )?;
        }
    }
    let inspection = fixture.library.organization_copy_inspect(
        review.id,
        selection,
        fixture.destination.clone(),
        (identity, volume),
    )?;
    if !inspection.all_published_bytes_verified || !fixture.preserved()? {
        return Err(failed(
            "Cross-volume source/destination preservation failed.",
        ));
    }
    let mut interruptions = Vec::new();
    for fault in [
        crate::executor::CopyFault::AfterIntent,
        crate::executor::CopyFault::AfterStageCreation,
        crate::executor::CopyFault::AfterCopyChunk,
        crate::executor::CopyFault::AfterFlush,
        crate::executor::CopyFault::AfterVerification,
        crate::executor::CopyFault::BeforePublish,
        crate::executor::CopyFault::AfterPublish,
        crate::executor::CopyFault::AfterItemCommit,
        crate::executor::CopyFault::AfterPlanCommit,
    ] {
        let stopped = Fixture::with_media_at(
            true,
            generated_photo_bytes()?,
            Some(&scope.destination_parent),
        )?;
        if stopped.plan.source.volume == stopped.plan.destination.volume {
            return Err(failed(
                "Interrupted fixture lost its actual cross-volume boundary.",
            ));
        }
        let journal = stopped.journal()?;
        let outcome = stopped.execute(&journal, Some(fault))?;
        if outcome != crate::executor::CopyOutcome::Interrupted || !stopped.preserved()? {
            return Err(failed(
                "Cross-volume interruption did not retain generated originals and neighbor.",
            ));
        }
        drop(journal);
        let journal = stopped.journal()?;
        let before = stopped.inspect(&journal)?;
        let replay_blocked = stopped.execute(&journal, None).is_err();
        if !replay_blocked
            || stopped.resume(&journal, None)? != crate::executor::CopyOutcome::CompletedVerified
            || stopped
                .inspect(&journal)?
                .iter()
                .any(|state| *state != crate::executor::RecoveredItem::PublishedVerified)
            || !stopped.preserved()?
        {
            return Err(failed(
                "Cross-volume fresh approved continuation failed preservation or complete-byte inspection.",
            ));
        }
        interruptions.push(serde_json::json!({"fault":fault,"before":before,"replayBlocked":true,"freshApprovedContinuation":"completed_verified","sourcesRetained":true,"destinationOnlyRetained":true}));
    }
    Ok(
        serde_json::json!({"policy":"owned-native-cross-volume-v1","nativeOs":std::env::consts::OS,"sourceVolume":fixture.plan.source_volume,"destinationVolume":fixture.plan.destination_volume,"sourceNativeDevice":fixture.plan.source.volume.to_string(),"destinationNativeDevice":fixture.plan.destination.volume.to_string(),"distinctVolumes":true,"jobs":jobs,"inspection":inspection,"interruptions":interruptions,"sourcesRetained":true,"destinationOnlyRetained":true,"originalMutationsEnabled":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cross_volume_scope_refuses_same_volume_and_forged_records_before_fixture_writes() {
        for variant in 0..4 {
            let workspace = tempfile::tempdir().unwrap();
            let id = uuid::Uuid::new_v4().to_string();
            let directory = workspace.path().join(".context/cross-volume").join(&id);
            std::fs::create_dir_all(&directory).unwrap();
            let image = directory.join("destination.vhdx");
            std::fs::write(&image, []).unwrap();
            let parent = workspace
                .path()
                .join(format!("stillharbour-cross-volume-{id}"));
            std::fs::create_dir(&parent).unwrap();
            std::fs::write(parent.join(".stillharbour-fixture-owner"), id.as_bytes()).unwrap();
            let mut record = serde_json::json!({"version":1,"runId":id,"image":image,"destinationParent":parent});
            match variant {
                1 => record["runId"] = serde_json::json!(uuid::Uuid::new_v4().to_string()),
                2 => {
                    let outside = workspace.path().join("outside.vhdx");
                    std::fs::write(&outside, []).unwrap();
                    record["image"] = serde_json::json!(outside);
                }
                3 => {
                    record["instructions"] =
                        serde_json::json!("Untrusted scope data is not executable.")
                }
                _ => {}
            }
            std::fs::write(
                directory.join("scope.json"),
                serde_json::to_vec(&record).unwrap(),
            )
            .unwrap();
            let error = checked_scope(&id, workspace.path())
                .err()
                .expect("A same-volume or forged scope must fail closed");
            if variant == 0 {
                assert!(error.message.contains("different native volume"));
            }
            assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 1);
            assert_eq!(
                std::fs::read(parent.join(".stillharbour-fixture-owner")).unwrap(),
                id.as_bytes()
            );
        }
    }
}
