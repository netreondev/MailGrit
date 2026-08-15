//! Application state: the current screen, session data, and bulk operation data.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::audit_ui::AuditWriter;
use crate::batch::BatchResult;
use crate::language::Language;
use crate::login_window::CookieInfo;
use crate::nav::DashboardSection;
use crate::theme::Theme;
use mailgrit_core_domain::{EditableUserRow, OperationTarget, PasswordGenerator};
use std::sync::Arc;
use zeroize::{Zeroize, Zeroizing};

/// Application state (which screen to show + data).
#[derive(Clone)]
pub struct AppState {
    /// Current screen.
    pub screen: Screen,
    /// Active top-level panel section (Operations/Audit). Default `Operations`.
    pub section: DashboardSection,
    /// Base URL of the iRedAdmin server.
    pub base_url: String,
    /// URL entered by the user (before validation).
    pub url_input: String,
    /// Session status.
    pub auth_status: AuthStatus,
    /// UI theme (dark/light), persisted to config.toml.
    pub theme: Theme,
    /// UI language, persisted to config.toml. Default is English.
    pub language: Language,
    /// Session is active (confirmed by the webview navigating to `/dashboard`).
    pub session_ok: bool,
    /// Session cookie name (from config.toml; default webpy_session_id). Informational.
    pub session_cookie_name: String,
    /// Everything CSV/bulk-operation related (the loaded data, the target and
    /// profile it is parsed against, the editable layer, and the last result).
    /// Grouped into a sub-state: AppState used to be a flat bag of 27 pub
    /// fields mixing routing, auth, CSV, audit, and modals.
    pub csv: CsvState,
    /// Audit log (hash-chained).
    pub audit: Option<Arc<AuditWriter>>,
    /// Current operation execution status.
    pub op_status: OpStatus,
    /// Awaiting-confirmation modal flags (delete, regenerate-all-passwords, master password).
    pub modals: ModalState,
    /// Recent audit entries to display.
    pub audit_entries: Vec<AuditEntryView>,
    /// Diagnostics panel: all cookies after the last extraction (for login debugging).
    pub last_cookies: Vec<CookieInfo>,
    /// Error message (if any).
    pub error_msg: Option<String>,
    /// Local password-check policy for the strength indicator in the table.
    pub password_policy: mailgrit_core_domain::PasswordPolicy,
    /// Password generator settings for auto-placement in the editable table.
    pub password_generator: PasswordGenerator,
    /// Entered master password (protects the audit/export key via the Argon2 KDF).
    /// `None` until the user enters it via the modal; stored only in memory,
    /// wrapped in [`Zeroizing`] so it is wiped when replaced/cleared.
    pub master_password: Option<Zeroizing<String>>,
    /// An unlock attempt (Argon2id in a background task) is in flight. Blocks
    /// repeated submissions from the modal and disables its buttons.
    pub unlock_pending: bool,
    /// Master password input fields in the modal (twice, for confirmation on creation).
    pub master_password_input: String,
    /// Confirmation field for the master password (must match `master_password_input`).
    pub master_password_confirm: String,
    /// Export lifecycle status flags (format-picker modal, await-unlock, in-progress).
    pub export: ExportState,
    /// Selected export encryption mode (true = encrypted, the default).
    pub export_encrypt: bool,
}

/// CSV / bulk-operation sub-state of [`AppState`].
#[derive(Clone)]
pub struct CsvState {
    /// Loaded CSV (the parse result).
    pub rows: Option<Arc<mailgrit_core_csv::ParsedCsv>>,
    /// Editable table rows (the plain-`String` layer). `None` until the CSV is
    /// loaded or when the target changes.
    pub editable_rows: Option<Vec<EditableUserRow>>,
    /// Current bulk operation target: User/Domain/Admin. Default `User`.
    pub current_target: OperationTarget,
    /// Operation profile the CSV is parsed against. `None` = default
    /// `for_user_create`.
    pub current_profile: Option<Arc<mailgrit_core_domain::OperationProfile>>,
    /// Mapping of CSV columns to profile fields. `None` until auto-detected.
    pub column_mapping: Option<Arc<mailgrit_core_csv::ColumnMapping>>,
    /// Result of the last bulk operation.
    pub batch_result: Option<Arc<BatchResult>>,
}

impl Default for CsvState {
    fn default() -> Self {
        Self {
            rows: None,
            editable_rows: None,
            current_target: OperationTarget::User,
            current_profile: None,
            column_mapping: None,
            batch_result: None,
        }
    }
}

impl CsvState {
    /// Clears the loaded data (keeping the selected target).
    pub fn clear_data(&mut self) {
        self.rows = None;
        self.column_mapping = None;
        self.current_profile = None;
        self.editable_rows = None;
    }
}

/// Awaiting-confirmation modal flags. Grouped (3 bools) to stay below clippy's
/// `struct_excessive_bools` limit; each flag independently gates one modal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModalState {
    /// Delete confirmation (fail-closed): awaits explicit confirmation.
    pub pending_delete: bool,
    /// "Regenerate all passwords" confirmation (irreversible): awaits a modal.
    pub pending_password_regenerate: bool,
    /// Awaits master password entry via the modal (to unlock audit/export).
    pub pending_master_password: bool,
}

/// Export lifecycle status flags. Grouped (3 bools) to stay below clippy's
/// `struct_excessive_bools` limit. The selected encryption mode
/// (`AppState::export_encrypt`) is a separate setting, not a lifecycle status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExportState {
    /// Opens the export format picker modal (encrypted/plaintext).
    pub pending_export_choice: bool,
    /// Intent for an encrypted export awaiting master password entry.
    /// Set in [`do_export`](crate::ops_export::do_export) when the master password
    /// has not been entered yet; after a successful unlock, the export resumes.
    pub pending_export_after_unlock: bool,
    /// Export is being performed by a background task (dialog + KDF + file write).
    /// Blocks repeated export button presses / disables the UI for the duration.
    /// Toggle ONLY via [`begin`](Self::begin)/[`finish`](Self::finish).
    pub export_in_progress: bool,
}

impl ExportState {
    /// Marks an export as running (the single way to set the flag — previously
    /// 6 call sites flipped the raw bool by hand).
    pub const fn begin(&mut self) {
        self.export_in_progress = true;
    }

    /// Clears the running flag (completion, cancel, or failure).
    pub const fn finish(&mut self) {
        self.export_in_progress = false;
    }
}

/// Audit entry for display in the UI (without the raw hash bytes).
#[derive(Clone)]
pub struct AuditEntryView {
    /// Entry timestamp.
    pub timestamp: String,
    /// Action (create/edit/delete).
    pub action: String,
    /// Details (human-readable).
    pub detail: String,
}

/// Application screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// iRedAdmin login window.
    Login,
    /// Native operations panel after authentication.
    Dashboard,
}

/// iRedAdmin session status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthStatus {
    /// No session extracted / the user has not started logging in.
    #[default]
    None,
    /// Login window open, awaiting iRedAdmin navigation after authentication.
    AwaitingLogin,
    /// Session obtained and confirmed.
    Connected,
}

/// Background operation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpStatus {
    /// Nothing running.
    Idle,
    /// A bulk operation is in progress.
    Running,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Creates the initial state on the login screen.
    ///
    /// The audit log is NOT opened here: the audit key is protected by the master
    /// password (Argon2id), which has not been entered yet. Unlocking happens in
    /// a background task after the password is entered in the UI modal (see
    /// [`complete_unlock`](Self::complete_unlock)).
    #[must_use]
    pub fn new() -> Self {
        // Configuration from TOML (no recompilation); defaults when absent.
        let config = crate::settings::load_or_create();
        let password_policy = config.password_policy.to_policy();
        // Normalize the generator against the server policy: forcibly enable the
        // required character classes (require_*) and clamp the length to >= min_len.
        let mut password_generator = config.password_generator.to_generator();
        password_generator.classes.set_uppercase(
            password_generator.classes.uppercase() | password_policy.classes.uppercase(),
        );
        password_generator.classes.set_lowercase(
            password_generator.classes.lowercase() | password_policy.classes.lowercase(),
        );
        password_generator
            .classes
            .set_digits(password_generator.classes.digits() | password_policy.classes.digits());
        password_generator
            .classes
            .set_special(password_generator.classes.special() | password_policy.classes.special());
        // No lower than the policy min_len and no higher than the UI ceiling.
        password_generator.length = password_generator
            .length
            .max(password_policy.min_len)
            .min(mailgrit_core_domain::password_gen::UI_MAX_LENGTH);
        Self {
            screen: Screen::Login,
            section: DashboardSection::default(),
            base_url: String::new(),
            url_input: config.base_url.clone(),
            auth_status: AuthStatus::None,
            theme: Theme::from_config(&config.theme),
            language: Language::from_config(&config.language),
            session_ok: false,
            session_cookie_name: config.session_cookie_name.clone(),
            csv: CsvState::default(),
            audit: None,
            op_status: OpStatus::Idle,
            modals: ModalState::default(),
            audit_entries: Vec::new(),
            last_cookies: Vec::new(),
            error_msg: None,
            password_policy,
            password_generator,
            master_password: None,
            unlock_pending: false,
            master_password_input: String::new(),
            master_password_confirm: String::new(),
            export: ExportState::default(),
            export_encrypt: true,
        }
    }

    /// Applies a successful audit unlock: stores the password in memory (for
    /// export), closes the modal, wipes the input fields, and populates
    /// `audit`/`audit_entries`.
    ///
    /// Split out of the old synchronous `unlock_audit`: the Argon2id KDF inside
    /// `AuditWriter::open` is memory-hard (64 MiB, t=3) and now runs in a
    /// `spawn_blocking` task (see `confirm_master_password`); this method only
    /// applies the result on the UI thread.
    pub fn complete_unlock(&mut self, master_password: Zeroizing<String>, audit: AuditWriter) {
        self.master_password = Some(master_password);
        self.modals.pending_master_password = false;
        self.unlock_pending = false;
        self.master_password_input.zeroize();
        self.master_password_confirm.zeroize();
        self.audit = Some(Arc::new(audit));
        self.refresh_audit();
    }

    /// Ends the iRedAdmin session and clears everything tied to it.
    ///
    /// Single source of truth for BOTH paths — manual logout and automatic
    /// session-loss: the two hand-maintained field lists had already diverged
    /// (session-loss forgot `column_mapping`, `current_profile`,
    /// `editable_rows`). The local audit log and the display entries stay (the
    /// audit belongs to the app, not to the server session). The caller decides
    /// what goes into `error_msg`.
    pub fn reset_session(&mut self) {
        self.op_status = OpStatus::Idle;
        self.session_ok = false;
        self.auth_status = AuthStatus::None;
        self.screen = Screen::Login;
        self.csv.clear_data();
        self.csv.batch_result = None;
        // Wipe the master password together with the session (Zeroizing drop).
        self.master_password = None;
        self.modals.pending_delete = false;
        self.modals.pending_password_regenerate = false;
    }

    /// Refreshes the list of audit entries from the writer.
    pub fn refresh_audit(&mut self) {
        if let Some(audit) = &self.audit {
            match audit.recent(10) {
                Ok(entries) => {
                    self.audit_entries = entries
                        .into_iter()
                        .map(|e| AuditEntryView {
                            timestamp: e.timestamp,
                            action: e.action,
                            detail: String::from_utf8_lossy(&e.payload).into_owned(),
                        })
                        .collect();
                }
                Err(e) => tracing::warn!("reading audit: {e}"),
            }
        }
    }

    /// Effective operation profile: the currently selected one, or the default `for_user_create`.
    #[must_use]
    pub fn effective_profile(&self) -> Arc<mailgrit_core_domain::OperationProfile> {
        self.csv
            .current_profile
            .clone()
            .unwrap_or_else(|| Arc::new(mailgrit_core_domain::OperationProfile::for_user_create()))
    }

    /// Sets the bulk operation target and the matching default create profile.
    /// On target change, the CSV and editable rows are reset.
    pub fn set_current_target(&mut self, target: OperationTarget) {
        self.csv.current_target = target;
        self.csv.current_profile = Some(Arc::new(match target {
            OperationTarget::User => mailgrit_core_domain::OperationProfile::for_user_create(),
            OperationTarget::Domain => mailgrit_core_domain::OperationProfile::for_domain_create(),
            OperationTarget::Admin => mailgrit_core_domain::OperationProfile::for_admin_create(),
        }));
        self.csv.clear_data();
    }
}
