use anyhow::Result;
use nostr_sdk::prelude::*;

use crate::nostr::messages::{InboundMessage, OutboundMessage};
use crate::state::SharedState;

/// Start listening for NIP-59 Gift Wrap messages addressed to the EC.
///
/// Subscribes to Kind::GiftWrap events targeting the EC's public key,
/// unwraps each message, parses the JSON content, and dispatches to
/// the appropriate handler. Replies are sent back via Gift Wrap.
pub async fn listen(state: SharedState) -> Result<()> {
    let ec_pubkey = state.ec_nostr_keys.public_key();

    // Subscribe to Gift Wrap events addressed to the EC.
    // limit(0) because NIP-59 tweaks timestamps, so historical fetch is unreliable.
    let filter = Filter::new()
        .kind(Kind::GiftWrap)
        .pubkey(ec_pubkey)
        .limit(0);

    state
        .nostr_client
        .subscribe(filter, None)
        .await?;

    tracing::info!("Nostr listener subscribed for Gift Wrap messages");

    state
        .nostr_client
        .handle_notifications(|notification| {
            let state = state.clone();
            async move {
                if let RelayPoolNotification::Event { event, .. } = notification
                    && event.kind == Kind::GiftWrap
                    && let Err(e) = handle_gift_wrap(&state, &event).await
                {
                    tracing::warn!(error = %e, "Failed to process Gift Wrap message");
                }
                Ok(false)
            }
        })
        .await?;

    Ok(())
}

/// Unwrap a Gift Wrap event, parse the inner message, dispatch to handler,
/// and send the reply back via Gift Wrap.
async fn handle_gift_wrap(state: &SharedState, event: &Event) -> Result<()> {
    let unwrapped = state
        .nostr_client
        .unwrap_gift_wrap(event)
        .await?;

    let sender = unwrapped.sender;
    let content = &unwrapped.rumor.content;

    let response = match serde_json::from_str::<InboundMessage>(content) {
        Ok(msg) => dispatch(state, &sender, msg).await,
        Err(e) => {
            tracing::warn!(error = %e, "Invalid inbound message format");
            OutboundMessage::error("INVALID_MESSAGE", format!("Malformed request: {e}"))
        }
    };

    send_reply(state, &sender, &response).await?;

    Ok(())
}

/// Route an inbound message to the appropriate handler.
///
/// Handlers are implemented in Phase 5. For now, return a placeholder error.
async fn dispatch(
    _state: &SharedState,
    _sender: &PublicKey,
    msg: InboundMessage,
) -> OutboundMessage {
    // Phase 5 will fill in actual handler calls:
    //   InboundMessage::Register { .. } => handlers::register::handle(state, sender, ..).await
    //   InboundMessage::RequestToken { .. } => handlers::request_token::handle(state, sender, ..).await
    //   InboundMessage::CastVote { .. } => handlers::cast_vote::handle(state, sender, ..).await
    match msg {
        InboundMessage::Register { .. } => {
            OutboundMessage::error("NOT_IMPLEMENTED", "register handler not yet implemented")
        }
        InboundMessage::RequestToken { .. } => {
            OutboundMessage::error("NOT_IMPLEMENTED", "request-token handler not yet implemented")
        }
        InboundMessage::CastVote { .. } => {
            OutboundMessage::error("NOT_IMPLEMENTED", "cast-vote handler not yet implemented")
        }
    }
}

/// Send a reply to a voter via NIP-59 Gift Wrap.
async fn send_reply(
    state: &SharedState,
    receiver: &PublicKey,
    response: &OutboundMessage,
) -> Result<()> {
    let content = serde_json::to_string(response)?;

    let rumor = EventBuilder::text_note(content)
        .build(state.ec_nostr_keys.public_key());

    state
        .nostr_client
        .gift_wrap(receiver, rumor, [])
        .await?;

    Ok(())
}
