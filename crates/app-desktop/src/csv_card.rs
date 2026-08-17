//! The CSV card of the Operations section: file choosing, summary, mapping,
//! the editable table, and password controls.
//!
//! Extracted from `operations_view.rs` to keep each file under the 400-line spec.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::components::button::{Button, ButtonKind, ButtonSize};
use crate::components::card::Card;
use crate::components::icon::{Icon, IconView};
use crate::csv_summary::CsvSummary;
use crate::editable_table_view::editable_table_view;
use crate::password_controls::password_controls_view;
use crate::screens::csv_load::load_csv_file;
use crate::state::{AppState, OpStatus};
use crate::views::{failed_csv_rows_view, mapping_panel_view};
use dioxus::prelude::*;

/// The "Operations" section: CSV + bulk operations + result.
///
/// Reads `state` via context (like `dashboard_screen`). The default section
/// `Operations` → behavior identical to Phase 14 on entry.
/// Card 1: CSV upload + mapping panel + editable table + password controls +
/// rejected rows.
pub fn csv_card(state: Signal<AppState>) -> Element {
    let op_status = state.read().op_status;
    // Read the language for re-rendering localized strings.
    crate::i18n::subscribe_to_language(state);
    let csv_summary = state
        .read()
        .csv
        .rows
        .as_ref()
        .map(|c| CsvSummary::from_parsed(c));
    let rejected_text = csv_summary.as_ref().and_then(|summary| {
        (summary.failed > 0).then(|| format!("{} {}", summary.failed, tr!("csv.rejected")))
    });
    rsx! {
        Card { data_card: "csv".to_string(),
            h2 { IconView { icon: Icon::Upload } {tr!("csv.card_title")} }
            p { class: "muted", {tr!("csv.format_hint")} }

            div { class: "op-row",
                Button {
                    kind: ButtonKind::Secondary,
                    size: ButtonSize::Small,
                    icon_left: Some(Icon::Upload),
                    disabled: op_status == OpStatus::Running,
                    onclick: move |_| {
                        // Native file-selection dialog via AsyncFileDialog:
                        // the blocking part runs on a separate thread (rfd on
                        // Windows spawns it itself), so the UI thread does not
                        // reenter the Dioxus runtime — just like in export.
                        // Parsing and state update happen after the path
                        // returns.
                        let s = state;
                        spawn(async move {
                            let title = tr!("csv.file_dialog_title");
                            let handle = rfd::AsyncFileDialog::new()
                                .add_filter("CSV", &["csv"])
                                .set_title(title)
                                .pick_file()
                                .await;
                            if let Some(handle) = handle {
                                let path = handle.path().to_path_buf();
                                load_csv_file(&s, &path);
                            }
                        });
                    },
                    {tr!("csv.choose_file")}
                }
            }

            // Flexible column-mapping panel.
            {mapping_panel_view(state, csv_summary.as_ref())}

            if let Some(summary) = &csv_summary {
                div { class: "dash-stat-row",
                    span { class: "dash-stat", "{summary.valid}" }
                    span { class: "dash-stat-label", {tr!("csv.valid_rows")} }
                }
                if let Some(rej) = &rejected_text {
                    p { class: "muted", "{rej}" }
                }
            } else {
                p { class: "muted mt-3", {tr!("csv.not_loaded")} }
            }

            // Password-generation controls + the editable row table.
            {password_controls_view(state)}
            {editable_table_view(state)}

            // The rejected-CSV-rows table.
            {failed_csv_rows_view(&state)}
        }
    }
}
