use anyhow::{Context, Result};
use nostr_sdk::prelude::*;
use serde_json::json;

use crate::counting::CountResult;
use crate::types::{Candidate, Election};

/// Nostr event kind for election announcements (addressable, replaceable by `d` tag).
const ELECTION_EVENT_KIND: u16 = 35_000;

/// Nostr event kind for election results / live tally updates.
const RESULT_EVENT_KIND: u16 = 35_001;

/// Publish (or replace) the election announcement event (Kind 35000).
///
/// Uses the election ID as the `d` tag so relays keep only the latest version.
pub async fn publish_election_event(
    client: &Client,
    election: &Election,
    candidates: &[Candidate],
) -> Result<EventId> {
    let candidate_list: Vec<serde_json::Value> = candidates
        .iter()
        .map(|c| json!({ "id": c.id, "name": c.name }))
        .collect();

    let content = json!({
        "election_id": election.id,
        "name": election.name,
        "start_time": election.start_time,
        "end_time": election.end_time,
        "status": election.status,
        "rules_id": election.rules_id,
        "rsa_pub_key": election.rsa_pub_key,
        "candidates": candidate_list,
    });

    let builder = EventBuilder::new(Kind::Custom(ELECTION_EVENT_KIND), content.to_string())
        .tag(Tag::identifier(&election.id));

    let output = client
        .send_event_builder(builder)
        .await
        .context("Failed to publish election event to relay")?;

    tracing::info!(
        election_id = %election.id,
        event_id = %output.id(),
        "Published Kind {} election event",
        ELECTION_EVENT_KIND
    );

    Ok(*output.id())
}

/// Publish the election result event (Kind 35001).
///
/// Uses the election ID as the `d` tag so relays keep only the latest tally.
pub async fn publish_result_event(
    client: &Client,
    election: &Election,
    result: &CountResult,
) -> Result<EventId> {
    let tally: Vec<serde_json::Value> = result
        .tally
        .iter()
        .map(|t| {
            json!({
                "candidate_id": t.candidate_id,
                "votes": t.votes,
                "status": t.status.as_str(),
            })
        })
        .collect();

    let elected: Vec<u8> = result.elected.clone();

    let mut content = json!({
        "election_id": election.id,
        "name": election.name,
        "rules_id": election.rules_id,
        "elected": elected,
        "tally": tally,
    });

    if let Some(ref count_sheet) = result.count_sheet {
        let rounds: Vec<serde_json::Value> = count_sheet
            .iter()
            .map(|r| {
                let round_tallies: Vec<serde_json::Value> = r
                    .tallies
                    .iter()
                    .map(|t| {
                        json!({
                            "candidate_id": t.candidate_id,
                            "votes": t.votes,
                            "status": t.status.as_str(),
                        })
                    })
                    .collect();
                json!({
                    "round": r.round,
                    "action": r.action,
                    "tallies": round_tallies,
                })
            })
            .collect();
        content["count_sheet"] = json!(rounds);
    }

    let builder = EventBuilder::new(Kind::Custom(RESULT_EVENT_KIND), content.to_string())
        .tag(Tag::identifier(&election.id));

    let output = client
        .send_event_builder(builder)
        .await
        .context("Failed to publish result event to relay")?;

    tracing::info!(
        election_id = %election.id,
        event_id = %output.id(),
        "Published Kind {} result event",
        RESULT_EVENT_KIND
    );

    Ok(*output.id())
}
