//! Keeper suggestions and mandatory guards. Neither result grants mutation.
use super::{invalid, model::MatchMode};
use crate::domain::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum KeeperOrder {
    Manual,
    OldestModified,
    NewestModified,
    OldestCreated,
    NewestCreated,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum KeeperBlockReason {
    Protected,
    NoAccessibleKeeper,
    OnlyLinkedKeeper,
    FreshBytesRequired,
    IncompleteBundle,
    UniqueCompanion,
    NotExactContent,
    UnknownFilesystemDate,
    ManualKeeperRequired,
    SelectedUnavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KeeperBlock {
    pub asset_id: String,
    pub reason: KeeperBlockReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeeperReviewInput {
    pub job_id: String,
    pub group_id: String,
    pub order: KeeperOrder,
    pub manual_keepers: Vec<String>,
    pub protected_folders: Vec<String>,
    pub preferred_folders: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KeeperReview {
    pub id: String,
    pub input: KeeperReviewInput,
    pub keepers: Vec<String>,
    pub suggestion_block: Option<KeeperBlockReason>,
    pub blocks: Vec<KeeperBlock>,
    pub explanation: String,
    pub mutation_available: bool,
}
pub(crate) struct Member {
    pub id: String,
    pub object: String,
    pub online: bool,
    pub protected: bool,
    pub preferred_rank: u32,
    pub modified_millis: Option<i64>,
    pub created_millis: Option<i64>,
    pub bundle_complete: bool,
    pub unique_companion: bool,
}

pub(crate) fn suggest(
    members: &[Member],
    order: KeeperOrder,
    manual: &[String],
) -> Result<std::result::Result<Vec<String>, KeeperBlockReason>> {
    if manual
        .iter()
        .any(|id| !members.iter().any(|member| &member.id == id))
    {
        return Err(invalid("A manual keeper is outside this reviewed group."));
    }
    let mut retained = members
        .iter()
        .filter(|member| member.protected || manual.contains(&member.id))
        .map(|member| member.id.clone())
        .collect::<Vec<_>>();
    retained.sort();
    retained.dedup();
    if !retained.is_empty() {
        return Ok(Ok(retained));
    }
    if order == KeeperOrder::Manual {
        return Ok(Err(KeeperBlockReason::ManualKeeperRequired));
    }
    let mut available = members
        .iter()
        .filter(|member| member.online)
        .collect::<Vec<_>>();
    let Some(rank) = available.iter().map(|member| member.preferred_rank).min() else {
        return Ok(Err(KeeperBlockReason::NoAccessibleKeeper));
    };
    available.retain(|member| member.preferred_rank == rank);
    let date = |member: &Member| match order {
        KeeperOrder::OldestCreated | KeeperOrder::NewestCreated => member.created_millis,
        _ => member.modified_millis,
    };
    if available.iter().any(|member| date(member).is_none()) {
        return Ok(Err(KeeperBlockReason::UnknownFilesystemDate));
    }
    available.sort_by(|a, b| {
        let comparison = date(a).cmp(&date(b));
        let comparison = if matches!(
            order,
            KeeperOrder::NewestCreated | KeeperOrder::NewestModified
        ) {
            comparison.reverse()
        } else {
            comparison
        };
        comparison.then_with(|| a.id.cmp(&b.id))
    });
    Ok(Ok(vec![available[0].id.clone()]))
}

/// `fresh_pairs` is supplied only by backend complete-byte verification. A hash
/// report, imported payload or renderer checkbox cannot populate this proof.
pub(crate) fn review_selection(
    members: &[Member],
    selected: &BTreeSet<String>,
    mode: MatchMode,
    fresh_pairs: &BTreeSet<(String, String)>,
) -> Result<Vec<KeeperBlock>> {
    if selected
        .iter()
        .any(|id| !members.iter().any(|member| &member.id == id))
    {
        return Err(invalid("A selected member is outside this reviewed group."));
    }
    let survivors = members
        .iter()
        .filter(|member| member.online && !selected.contains(&member.id))
        .collect::<Vec<_>>();
    let mut blocked = Vec::new();
    for member in members
        .iter()
        .filter(|member| selected.contains(&member.id))
    {
        let mut reasons = Vec::new();
        if !member.online {
            reasons.push(KeeperBlockReason::SelectedUnavailable);
        }
        if member.protected {
            reasons.push(KeeperBlockReason::Protected);
        }
        if mode != MatchMode::ExactContent {
            reasons.push(KeeperBlockReason::NotExactContent);
        }
        if !member.bundle_complete {
            reasons.push(KeeperBlockReason::IncompleteBundle);
        }
        if member.unique_companion {
            reasons.push(KeeperBlockReason::UniqueCompanion);
        }
        let independent = survivors
            .iter()
            .filter(|keeper| {
                !keeper.object.is_empty()
                    && !member.object.is_empty()
                    && keeper.object != member.object
            })
            .collect::<Vec<_>>();
        if survivors.is_empty() {
            reasons.push(KeeperBlockReason::NoAccessibleKeeper);
        } else if independent.is_empty() {
            reasons.push(KeeperBlockReason::OnlyLinkedKeeper);
        } else if !independent
            .iter()
            .any(|keeper| fresh_pairs.contains(&(member.id.clone(), keeper.id.clone())))
        {
            reasons.push(KeeperBlockReason::FreshBytesRequired);
        }
        blocked.extend(reasons.into_iter().map(|reason| KeeperBlock {
            asset_id: member.id.clone(),
            reason,
        }));
    }
    Ok(blocked)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn member(id: &str) -> Member {
        Member {
            id: id.into(),
            object: id.into(),
            online: true,
            protected: false,
            preferred_rank: 1,
            modified_millis: Some(0),
            created_millis: None,
            bundle_complete: true,
            unique_companion: false,
        }
    }
    #[test]
    fn last_accessible_copy_protected_paths_and_link_aliases_cannot_be_selected_away() {
        let mut members = vec![member("a"), member("b"), member("offline")];
        members[0].protected = true;
        members[1].object = "a".into();
        members[2].online = false;
        let selected = BTreeSet::from(["a".into()]);
        let proofs = BTreeSet::from([("a".into(), "b".into())]);
        let blocked =
            review_selection(&members, &selected, MatchMode::ExactContent, &proofs).unwrap();
        assert!(
            blocked
                .iter()
                .any(|block| block.reason == KeeperBlockReason::Protected)
        );
        assert!(
            blocked
                .iter()
                .any(|block| block.reason == KeeperBlockReason::OnlyLinkedKeeper)
        );
        let selected = BTreeSet::from(["a".into(), "b".into()]);
        assert!(
            review_selection(&members, &selected, MatchMode::ExactContent, &proofs)
                .unwrap()
                .iter()
                .any(|block| block.reason == KeeperBlockReason::NoAccessibleKeeper)
        );
    }
    #[test]
    fn hash_only_property_pixel_and_incomplete_or_unique_bundles_never_qualify() {
        let mut members = vec![member("a"), member("b")];
        let selected = BTreeSet::from(["b".into()]);
        assert_eq!(
            review_selection(
                &members,
                &selected,
                MatchMode::ExactContent,
                &BTreeSet::new()
            )
            .unwrap()[0]
                .reason,
            KeeperBlockReason::FreshBytesRequired
        );
        let proofs = BTreeSet::from([("b".into(), "a".into())]);
        for mode in [MatchMode::Properties, MatchMode::NormalizedPixels] {
            assert!(
                review_selection(&members, &selected, mode, &proofs)
                    .unwrap()
                    .iter()
                    .any(|block| block.reason == KeeperBlockReason::NotExactContent)
            );
        }
        members[1].bundle_complete = false;
        members[1].unique_companion = true;
        let blocked =
            review_selection(&members, &selected, MatchMode::ExactContent, &proofs).unwrap();
        assert!(
            blocked
                .iter()
                .any(|block| block.reason == KeeperBlockReason::IncompleteBundle)
        );
        assert!(
            blocked
                .iter()
                .any(|block| block.reason == KeeperBlockReason::UniqueCompanion)
        );
        members[1].bundle_complete = true;
        members[1].unique_companion = false;
        assert!(
            review_selection(&members, &selected, MatchMode::ExactContent, &proofs)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn date_policies_do_not_invent_birthtime_and_manual_keeps_can_preserve_multiple() {
        let mut members = vec![member("a"), member("b")];
        members[1].modified_millis = Some(1);
        assert_eq!(
            suggest(&members, KeeperOrder::OldestCreated, &[]).unwrap(),
            Err(KeeperBlockReason::UnknownFilesystemDate)
        );
        assert_eq!(
            suggest(&members, KeeperOrder::NewestModified, &[])
                .unwrap()
                .unwrap(),
            vec!["b"]
        );
        members[0].preferred_rank = 0;
        assert_eq!(
            suggest(&members, KeeperOrder::NewestModified, &[])
                .unwrap()
                .unwrap(),
            vec!["a"]
        );
        assert_eq!(
            suggest(&members, KeeperOrder::Manual, &["a".into(), "b".into()])
                .unwrap()
                .unwrap()
                .len(),
            2
        );
        assert!(suggest(&members, KeeperOrder::Manual, &["foreign".into()]).is_err());
    }
}
