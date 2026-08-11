//! IPC dispatcher: routing async-JS responses to Rust via `window.ipc.postMessage`.
//!
//! `evaluate_script_with_callback` does NOT await a Promise (it synchronously
//! serializes the result; a Promise collapses to "{}"). So async JS (fetch) talks
//! to Rust via `window.ipc.postMessage("tag:id:json")` — wry delivers this to the
//! single `with_ipc_handler` of the webview. Here we keep a table of pending
//! requests keyed by id (correlation-id) to route responses to the right
//! oneshot receivers.
//!
//! Message format: `tag:id:json`, e.g. `sc:7:{"status":200,"isLogin":false}`
//! or `batch:12:[{...}]`.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// Reply from the JS side over the IPC channel.
#[derive(Debug)]
pub enum IpcReply {
    /// Operation batch result (`batch`).
    Batch(Vec<crate::webview_ops::OpResult>),
    /// Form diagnostics result (`diag`).
    Diag(String),
}

/// Table of pending IPC responses: id -> oneshot::Sender.
pub type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<IpcReply>>>>;

/// Correlation-id generator.
///
/// On a poisoned mutex we recover the inner value (`into_inner`) and continue,
/// rather than returning a hardcoded `1` that could collide with a real id=1 and
/// route a late IPC response to the wrong receiver. This is the same poison
/// recovery policy as in `login_window.rs` (HWND-bearing state).
fn next_id(counter: &Mutex<u64>) -> u64 {
    // Recover from the poisoned state (`into_inner`) and continue, rather than
    // returning a hardcoded `1` that could collide with a real id=1 and route a
    // late IPC response to the wrong receiver. Same approach as in
    // `login_window::handle_event` (match Err(poisoned) => into_inner).
    let mut c = match counter.lock() {
        Ok(c) => c,
        Err(poisoned) => poisoned.into_inner(),
    };
    *c = c.wrapping_add(1);
    *c
}

/// IPC dispatcher: registers a pending wait, returns (id, oneshot::Receiver).
/// The `id` is embedded into JS so the reply reaches the right receiver.
pub fn register(pending: &PendingMap, counter: &Mutex<u64>) -> (u64, oneshot::Receiver<IpcReply>) {
    let id = next_id(counter);
    let (tx, rx) = oneshot::channel::<IpcReply>();
    // Recover from the poisoned state (like in login_window); otherwise, on a
    // panic in another thread, tx would be dropped immediately and rx would get
    // a RecvError -> "0 results, webview failure", masking the true cause.
    // The guard is kept in a short block-scope (significant Drop).
    {
        let mut m = match pending.lock() {
            Ok(m) => m,
            Err(poisoned) => poisoned.into_inner(),
        };
        m.insert(id, tx);
    }
    tracing::debug!("IPC: registered pending id={id}");
    (id, rx)
}

/// Cancels a pending IPC request (removes its entry from the pending map).
/// Called on operation timeout or webview close — otherwise the entry would leak
/// (a late reply has no one to deliver to, the oneshot::Receiver is dropped).
pub fn cancel(pending: &PendingMap, id: u64) {
    let removed = {
        let mut m = match pending.lock() {
            Ok(m) => m,
            Err(poisoned) => poisoned.into_inner(),
        };
        m.remove(&id).is_some()
    };
    if removed {
        tracing::debug!("IPC: canceled pending id={id}");
    }
}

/// Parses the IPC message `tag:id:json` and delivers the reply to the pending map.
/// Called from the webview's ipc_handler on every `window.ipc.postMessage(...)`.
pub fn dispatch(pending: &PendingMap, body: &str) {
    tracing::debug!("IPC: message received (length {})", body.len());
    // Format: tag:id:json  (tag and id are up to the first two ':')
    let mut parts = body.splitn(3, ':');
    let tag = parts.next().unwrap_or("");
    let id_str = parts.next().unwrap_or("");
    let json = parts.next().unwrap_or("");
    let Some(id) = id_str.parse::<u64>().ok() else {
        tracing::warn!("IPC: invalid id in message: {body}");
        return;
    };
    // Recover from the poisoned state (like in login_window/register).
    // The guard is in a short block-scope, to release the lock before parsing
    // and sending (significant Drop), and not to hold the map during the heavy
    // parse_batch_result.
    let tx = {
        let mut m = match pending.lock() {
            Ok(m) => m,
            Err(poisoned) => poisoned.into_inner(),
        };
        m.remove(&id)
    };
    let Some(tx) = tx else {
        tracing::warn!("IPC: no pending wait for id={id} (tag={tag})");
        return;
    };
    let reply = match tag {
        "batch" => {
            let results = crate::webview_ops::parse_batch_result(json);
            tracing::info!("IPC batch id={id}: {} results", results.len());
            IpcReply::Batch(results)
        }
        "diag" => {
            tracing::info!("IPC diag id={id}: form diagnostics (length {})", json.len());
            IpcReply::Diag(json.to_string())
        }
        other => {
            tracing::warn!("IPC: unknown tag={other} id={id}");
            return;
        }
    };
    let _ = tx.send(reply);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_map() -> (PendingMap, std::sync::Mutex<u64>) {
        (
            Arc::new(Mutex::new(HashMap::new())),
            std::sync::Mutex::new(0),
        )
    }

    /// Locks the pending map, recovering the guard on a poisoned mutex (mirrors
    /// the production poison-recovery policy in `register`/`dispatch`).
    fn lock_map(
        pending: &PendingMap,
    ) -> std::sync::MutexGuard<'_, HashMap<u64, oneshot::Sender<IpcReply>>> {
        match pending.lock() {
            Ok(m) => m,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    // Full round-trip: register -> dispatch -> reply received.
    #[test]
    fn register_then_dispatch_delivers_reply() {
        let (pending, counter) = fresh_map();
        let (id, mut rx) = register(&pending, &counter);
        // A valid batch:id:json message.
        dispatch(
            &pending,
            r#"batch:ID:[{"username":"u","domain":"d","ok":true,"status":200}]"#
                .replace("ID", &id.to_string())
                .as_str(),
        );
        let reply = rx.try_recv();
        assert!(matches!(reply, Ok(IpcReply::Batch(r)) if r.len() == 1));
        // After dispatch the entry is removed from the map.
        assert!(lock_map(&pending).is_empty());
    }

    // Invalid id -> warning, entry untouched (but there is none anyway).
    #[test]
    fn dispatch_invalid_id_ignored() {
        let (pending, counter) = fresh_map();
        let (_id, mut rx) = register(&pending, &counter);
        dispatch(&pending, "batch:notanumber:[{}]");
        // Reply not delivered (would be Ok); the receiver still waits.
        assert!(rx.try_recv().is_err());
    }

    // Unknown tag -> entry removed from the map, reply not delivered.
    // (dispatch takes tx by id before matching on the tag; unknown tag -> tx dropped.)
    #[test]
    fn dispatch_unknown_tag_ignored() {
        let (pending, counter) = fresh_map();
        let (id, mut rx) = register(&pending, &counter);
        dispatch(&pending, &format!("unknown:{id}:[...]"));
        // Reply not delivered.
        assert!(rx.try_recv().is_err());
        // Entry removed (dispatch takes tx by id before matching on the tag).
        assert!(!lock_map(&pending).contains_key(&id));
    }

    // cancel removes the entry; a late dispatch does not panic.
    #[test]
    fn cancel_removes_entry() {
        let (pending, counter) = fresh_map();
        let (id, _rx) = register(&pending, &counter);
        assert_eq!(lock_map(&pending).len(), 1);
        cancel(&pending, id);
        assert!(lock_map(&pending).is_empty());
        // A late dispatch for the canceled id does not panic (warn).
        dispatch(&pending, &format!("batch:{id}:[{{}}]"));
    }

    // Registering multiple ids — counter increment.
    #[test]
    fn register_increments_id() {
        let (pending, counter) = fresh_map();
        let (id1, _) = register(&pending, &counter);
        let (id2, _) = register(&pending, &counter);
        assert_ne!(id1, id2);
        assert_eq!(lock_map(&pending).len(), 2);
    }

    // JSON containing a colon — splitn(3, ':') splits correctly.
    #[test]
    fn dispatch_json_with_colon() {
        let (pending, counter) = fresh_map();
        let (id, mut rx) = register(&pending, &counter);
        // JSON contains ':' (e.g. in an error string).
        let body = format!(
            r#"batch:{id}:[{{"username":"u","domain":"d","ok":false,"status":200,"error":"HTTP 200: detail"}}]"#
        );
        dispatch(&pending, &body);
        let reply = rx.try_recv();
        assert!(matches!(reply, Ok(IpcReply::Batch(r)) if r.len() == 1));
    }
}
