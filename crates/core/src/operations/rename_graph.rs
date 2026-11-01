//! Pure same-root rename/undo ordering. Native name facts are adapter inputs;
//! this module never opens files or supplies execution/recovery authority.
use crate::{
    domain::{AppError, ErrorCode, Result},
    organization::portable_filename,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Member {
    pub id: String,
    pub before: String,
    pub after: String,
    pub revision: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NameFact {
    /// Opaque equivalence class observed on the exact native parent. Never a
    /// lowercased or Unicode-normalized display name guessed by this planner.
    pub native_key: String,
    pub occupant: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Facts {
    pub root_id: String,
    pub root_revision: String,
    pub available: bool,
    pub targets: BTreeMap<String, NameFact>,
    pub stages: BTreeMap<String, NameFact>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Conflict {
    UnavailableRoot,
    MissingNativeNameEvidence,
    NativeNameCollision,
    OccupiedTarget,
    OccupiedStage,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct Issue {
    pub kind: Conflict,
    pub member_ids: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct Step {
    pub member_id: String,
    pub from: String,
    pub to: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Plan {
    pub policy: String,
    pub id: String,
    pub root_id: String,
    pub root_revision: String,
    pub name_facts: Facts,
    pub digest: String,
    pub members: Vec<Member>,
    pub stages: BTreeMap<String, String>,
    pub steps: Vec<Step>,
    pub issues: Vec<Issue>,
    pub execution_available: bool,
    pub independent_recovery_required: bool,
}
fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}
fn revision(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
pub(crate) fn stage_name(plan: &str, member: &str) -> String {
    format!(".stillharbour-rename-{plan}-{member}.partial")
}
pub(crate) fn build(id: String, mut members: Vec<Member>, facts: Facts) -> Result<Plan> {
    super::validate_id(&id)?;
    super::validate_id(&facts.root_id)?;
    if members.is_empty() || members.len() > 512 || !revision(&facts.root_revision) {
        return Err(invalid("Use a bounded, revision-bound rename scope."));
    }
    members.sort_by(|left, right| left.id.cmp(&right.id));
    let ids = members
        .iter()
        .map(|member| member.id.clone())
        .collect::<BTreeSet<_>>();
    if ids.len() != members.len()
        || members
            .iter()
            .map(|member| &member.before)
            .collect::<BTreeSet<_>>()
            .len()
            != members.len()
    {
        return Err(invalid("Each rename source must occur exactly once."));
    }
    for member in &members {
        super::validate_id(&member.id)?;
        if !portable_filename(&member.before)
            || !portable_filename(&member.after)
            || member.before.starts_with(".stillharbour-")
            || member.after.starts_with(".stillharbour-")
            || !revision(&member.revision)
        {
            return Err(invalid(
                "Unsupported rename leaf or source revision. Native locators remain backend owned.",
            ));
        }
    }
    let changed = members
        .iter()
        .filter(|member| member.before != member.after)
        .collect::<Vec<_>>();
    let movable = changed
        .iter()
        .map(|member| member.id.as_str())
        .collect::<BTreeSet<_>>();
    let stages = changed
        .iter()
        .map(|member| (member.id.clone(), stage_name(&id, &member.id)))
        .collect::<BTreeMap<_, _>>();
    if facts.targets.len() > members.len()
        || facts.stages.len() > changed.len()
        || facts.targets.keys().any(|id| !ids.contains(id))
        || facts
            .stages
            .keys()
            .any(|name| !stages.values().any(|stage| stage == name))
    {
        return Err(invalid(
            "Native name evidence includes entries outside this exact rename scope.",
        ));
    }
    let mut issues = Vec::new();
    if !facts.available {
        issues.push(Issue {
            kind: Conflict::UnavailableRoot,
            member_ids: ids.iter().cloned().collect(),
        });
    }
    let mut keys: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for member in &members {
        let Some(target) = facts.targets.get(&member.id) else {
            issues.push(Issue {
                kind: Conflict::MissingNativeNameEvidence,
                member_ids: vec![member.id.clone()],
            });
            continue;
        };
        if target.native_key.is_empty() || target.native_key.len() > 256 {
            return Err(invalid("Invalid native name-equivalence evidence."));
        }
        keys.entry(target.native_key.clone())
            .or_default()
            .push(member.id.clone());
        if target
            .occupant
            .as_ref()
            .is_some_and(|occupant| occupant != &member.id && !movable.contains(occupant.as_str()))
        {
            issues.push(Issue {
                kind: Conflict::OccupiedTarget,
                member_ids: vec![member.id.clone()],
            });
        }
        if let Some(stage) = stages.get(&member.id) {
            match facts.stages.get(stage) {
                Some(fact) if !fact.native_key.is_empty() && fact.native_key.len() <= 256 => {
                    keys.entry(fact.native_key.clone())
                        .or_default()
                        .push(member.id.clone());
                    if fact.occupant.is_some() {
                        issues.push(Issue {
                            kind: Conflict::OccupiedStage,
                            member_ids: vec![member.id.clone()],
                        });
                    }
                }
                _ => issues.push(Issue {
                    kind: Conflict::MissingNativeNameEvidence,
                    member_ids: vec![member.id.clone()],
                }),
            }
        }
    }
    for participants in keys.values().filter(|participants| participants.len() > 1) {
        let mut participants = participants.clone();
        participants.sort();
        participants.dedup();
        issues.push(Issue {
            kind: Conflict::NativeNameCollision,
            member_ids: participants,
        });
    }
    let mut steps = Vec::new();
    if issues.is_empty() {
        for member in &changed {
            steps.push(Step {
                member_id: member.id.clone(),
                from: member.before.clone(),
                to: stages[&member.id].clone(),
            });
        }
        for member in &changed {
            steps.push(Step {
                member_id: member.id.clone(),
                from: stages[&member.id].clone(),
                to: member.after.clone(),
            });
        }
    }
    let independent_recovery_required = !changed.is_empty();
    let mut plan = Plan {
        policy: "same-root-rename-graph-v1".into(),
        id,
        root_id: facts.root_id.clone(),
        root_revision: facts.root_revision.clone(),
        name_facts: facts,
        digest: String::new(),
        members,
        stages,
        steps,
        issues,
        execution_available: false,
        independent_recovery_required,
    };
    plan.digest = blake3::hash(
        &serde_json::to_vec(&plan)
            .map_err(|error| invalid("Could not fingerprint rename ordering.").caused_by(error))?,
    )
    .to_hex()
    .to_string();
    Ok(plan)
}
#[cfg(any(test, feature = "fixture-probes"))]
pub(crate) fn reverse(
    id: String,
    forward: &Plan,
    current_revisions: &BTreeMap<String, String>,
    facts: Facts,
) -> Result<Plan> {
    let reconstructed = build(
        forward.id.clone(),
        forward.members.clone(),
        forward.name_facts.clone(),
    )?;
    let encode = |plan: &Plan| {
        serde_json::to_vec(plan).map_err(|error| {
            invalid("Could not validate retained rename planning evidence.").caused_by(error)
        })
    };
    if encode(&reconstructed)? != encode(forward)? {
        return Err(invalid(
            "Retained rename planning evidence is not its exact canonical plan.",
        ));
    }
    if !forward.issues.is_empty()
        || forward.root_id != facts.root_id
        || current_revisions.len() != forward.members.len()
    {
        return Err(invalid(
            "Undo needs the complete retained operation and current native scope.",
        ));
    }
    let members = forward
        .members
        .iter()
        .map(|member| {
            Ok(Member {
                id: member.id.clone(),
                before: member.after.clone(),
                after: member.before.clone(),
                revision: current_revisions
                    .get(&member.id)
                    .cloned()
                    .ok_or_else(|| invalid("A current undo source revision is unavailable."))?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    build(id, members, facts)
}

/// Simulated name facts exercise pure ordering only. This is not native name
/// reservation, a recovery provider, an executable plan or mutation evidence.
#[cfg(any(test, feature = "fixture-probes"))]
pub(crate) fn fixture_preview_probe() -> Result<serde_json::Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let root = uuid::Uuid::new_v4().to_string();
    let members = [
        ("Case.JPG", "case.JPG"),
        ("A.JPG", "B.JPG"),
        ("B.JPG", "A.JPG"),
    ]
    .into_iter()
    .map(|(before, after)| Member {
        id: uuid::Uuid::new_v4().to_string(),
        before: before.into(),
        after: after.into(),
        revision: "a".repeat(64),
    })
    .collect::<Vec<_>>();
    let facts = Facts {
        root_id: root,
        root_revision: "b".repeat(64),
        available: true,
        targets: members
            .iter()
            .enumerate()
            .map(|(index, member)| {
                (
                    member.id.clone(),
                    NameFact {
                        native_key: format!("simulated-target-{index}"),
                        occupant: members
                            .iter()
                            .find(|source| source.before == member.after)
                            .map(|source| source.id.clone()),
                    },
                )
            })
            .collect(),
        stages: members
            .iter()
            .enumerate()
            .map(|(index, member)| {
                (
                    stage_name(&id, &member.id),
                    NameFact {
                        native_key: format!("simulated-stage-{index}"),
                        occupant: None,
                    },
                )
            })
            .collect(),
    };
    let plan = build(id, members, facts.clone())?;
    let reverse_id = uuid::Uuid::new_v4().to_string();
    let mut reverse_facts = facts;
    reverse_facts.stages = plan
        .members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            (
                stage_name(&reverse_id, &member.id),
                NameFact {
                    native_key: format!("simulated-undo-stage-{index}"),
                    occupant: None,
                },
            )
        })
        .collect();
    let undo = reverse(
        reverse_id,
        &plan,
        &plan
            .members
            .iter()
            .map(|member| (member.id.clone(), "c".repeat(64)))
            .collect(),
        reverse_facts,
    )?;
    Ok(
        serde_json::json!({"scope":"simulated-pure-planning-only","forward":plan,"reverse":undo,"nativeMutationTested":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rename_graph_stages_every_changed_source_before_any_publication_and_undo_reverses_names() {
        let probe = fixture_preview_probe().unwrap();
        let plan: Plan = serde_json::from_value(probe["forward"].clone()).unwrap();
        let undo: Plan = serde_json::from_value(probe["reverse"].clone()).unwrap();
        assert!(plan.issues.is_empty());
        assert_eq!(plan.steps.len(), 6);
        for (index, member) in plan.members.iter().enumerate() {
            assert_eq!(plan.steps[index].from, member.before);
            assert_eq!(plan.steps[index + 3].to, member.after);
            let restored = undo
                .members
                .iter()
                .find(|restored| restored.id == member.id)
                .unwrap();
            assert_eq!(restored.before, member.after);
            assert_eq!(restored.after, member.before);
        }
        assert!(!plan.execution_available && plan.independent_recovery_required);
        assert_eq!(probe["nativeMutationTested"], false);
    }
    #[test]
    fn rename_graph_unknown_offline_native_collisions_and_occupied_stages_have_no_steps() {
        let probe = fixture_preview_probe().unwrap();
        let plan: Plan = serde_json::from_value(probe["forward"].clone()).unwrap();
        for variant in 0..5 {
            let id = uuid::Uuid::new_v4().to_string();
            let mut facts = Facts {
                root_id: plan.root_id.clone(),
                root_revision: "d".repeat(64),
                available: true,
                targets: plan
                    .members
                    .iter()
                    .enumerate()
                    .map(|(index, member)| {
                        (
                            member.id.clone(),
                            NameFact {
                                native_key: format!("target-{index}"),
                                occupant: None,
                            },
                        )
                    })
                    .collect(),
                stages: plan
                    .members
                    .iter()
                    .enumerate()
                    .map(|(index, member)| {
                        (
                            stage_name(&id, &member.id),
                            NameFact {
                                native_key: format!("stage-{index}"),
                                occupant: None,
                            },
                        )
                    })
                    .collect(),
            };
            let expected = match variant {
                0 => {
                    facts.available = false;
                    Conflict::UnavailableRoot
                }
                1 => {
                    facts.targets.remove(&plan.members[0].id);
                    Conflict::MissingNativeNameEvidence
                }
                2 => {
                    facts
                        .targets
                        .get_mut(&plan.members[1].id)
                        .unwrap()
                        .native_key = "target-0".into();
                    Conflict::NativeNameCollision
                }
                3 => {
                    facts.targets.get_mut(&plan.members[0].id).unwrap().occupant =
                        Some("foreign-preserved-object".into());
                    Conflict::OccupiedTarget
                }
                _ => {
                    facts.stages.values_mut().next().unwrap().occupant =
                        Some(plan.members[0].id.clone());
                    Conflict::OccupiedStage
                }
            };
            let blocked = build(id, plan.members.clone(), facts).unwrap();
            assert!(blocked.steps.is_empty());
            assert!(blocked.issues.iter().any(|issue| issue.kind == expected));
        }
    }
    #[test]
    fn rename_graph_reverse_reports_new_occupant_of_old_name_without_overriding_it() {
        let probe = fixture_preview_probe().unwrap();
        let plan: Plan = serde_json::from_value(probe["forward"].clone()).unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let facts = Facts {
            root_id: plan.root_id.clone(),
            root_revision: "e".repeat(64),
            available: true,
            targets: plan
                .members
                .iter()
                .enumerate()
                .map(|(index, member)| {
                    (
                        member.id.clone(),
                        NameFact {
                            native_key: format!("old-target-{index}"),
                            occupant: if index == 0 {
                                Some("newer-preserved-object".into())
                            } else {
                                None
                            },
                        },
                    )
                })
                .collect(),
            stages: plan
                .members
                .iter()
                .enumerate()
                .map(|(index, member)| {
                    (
                        stage_name(&id, &member.id),
                        NameFact {
                            native_key: format!("undo-stage-{index}"),
                            occupant: None,
                        },
                    )
                })
                .collect(),
        };
        let undo = reverse(
            id,
            &plan,
            &plan
                .members
                .iter()
                .map(|member| (member.id.clone(), "f".repeat(64)))
                .collect(),
            facts,
        )
        .unwrap();
        assert!(undo.steps.is_empty());
        assert!(
            undo.issues
                .iter()
                .any(|issue| issue.kind == Conflict::OccupiedTarget)
        );
    }
    #[test]
    fn rename_graph_binds_name_facts_and_rejects_tampered_retained_steps() {
        let probe = fixture_preview_probe().unwrap();
        let plan: Plan = serde_json::from_value(probe["forward"].clone()).unwrap();
        let mut unordered = plan.members.clone();
        unordered.reverse();
        assert_eq!(
            build(plan.id.clone(), unordered, plan.name_facts.clone())
                .unwrap()
                .digest,
            plan.digest
        );
        let mut facts = plan.name_facts.clone();
        facts.targets.values_mut().next().unwrap().native_key = "different-native-proof".into();
        let changed = build(plan.id.clone(), plan.members.clone(), facts).unwrap();
        assert_eq!(plan.steps, changed.steps);
        assert_ne!(plan.digest, changed.digest);
        let mut bad = plan.clone();
        bad.steps[0].to = "wrong-stage".into();
        bad.digest.clear();
        bad.digest = blake3::hash(&serde_json::to_vec(&bad).unwrap())
            .to_hex()
            .to_string();
        let revisions = plan
            .members
            .iter()
            .map(|member| (member.id.clone(), "d".repeat(64)))
            .collect();
        assert!(
            reverse(
                uuid::Uuid::new_v4().to_string(),
                &bad,
                &revisions,
                plan.name_facts.clone()
            )
            .is_err()
        );
    }
    #[test]
    fn rename_graph_noops_need_no_staging_or_recovery_and_foreign_facts_are_refused() {
        let id = uuid::Uuid::new_v4().to_string();
        let member = Member {
            id: uuid::Uuid::new_v4().to_string(),
            before: "Photo.JPG".into(),
            after: "Photo.JPG".into(),
            revision: "a".repeat(64),
        };
        let facts = Facts {
            root_id: uuid::Uuid::new_v4().to_string(),
            root_revision: "b".repeat(64),
            available: true,
            targets: BTreeMap::from([(
                member.id.clone(),
                NameFact {
                    native_key: "same-file-name".into(),
                    occupant: Some(member.id.clone()),
                },
            )]),
            stages: BTreeMap::new(),
        };
        let plan = build(id.clone(), vec![member.clone()], facts.clone()).unwrap();
        assert!(
            plan.steps.is_empty()
                && plan.stages.is_empty()
                && !plan.independent_recovery_required
                && !plan.execution_available
        );
        let mut foreign = facts;
        foreign.targets.insert(
            uuid::Uuid::new_v4().to_string(),
            NameFact {
                native_key: "outside-scope".into(),
                occupant: None,
            },
        );
        assert!(build(id, vec![member], foreign).is_err());
    }
}
