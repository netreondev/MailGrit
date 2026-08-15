// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/**
 * Centralized MailGrit UI selectors.
 *
 * Tests operate on the real Dioxus DOM (RSX -> HTML). Selectors rely on stable
 * CSS classes and aria attributes (intentionally added/existing), rather than
 * on brittle structures. If a class changes, we fix it in one place.
 *
 * Structure:
 *  - `SEL`        — base selectors (brand, login screen, language selector,
 *                   theme, window buttons).
 *  - `DASH`       — dashboard selectors (context-bar, cards, section
 *                   navigation, table, password controls, operation buttons,
 *                   modals, audit). The dashboard is launched in E2E through
 *                   the env hook `MAILGRIT_E2E_DASHBOARD` (see fixtures/app.ts).
 *  - `LANGUAGES`  — 9 locales (mirror of the Rust `Language::all()`).
 *  - `I18N_MARKERS` — localized strings for verifying UI text.
 */

/** Titlebar + login screen + shared elements. */
export const SEL = {
  // Brand — exact selectors (an OR-fallback would silently retarget onto a
  // wrong element after a refactor instead of failing).
  titlebar: '.titlebar',
  titlebarLogo: 'svg.logo.titlebar-logo',
  titlebarName: '.titlebar-name',
  loginScreen: '.login-screen',
  loginLogo: 'svg.logo.login-logo',
  brandName: 'h1', // on the login screen — the only h1 with "MailGrit"

  // Login screen
  serverInput: '.login-screen input[type="url"]',
  openFormButton: '.login-screen .btn-primary',

  // Language selector
  langMenu: '.lang-menu',
  langTrigger: '.lang-trigger',
  langOverlay: '.lang-overlay',
  langDropdown: '.lang-dropdown',
  langItem: '.lang-item',
  langItemActive: '.lang-item-active',
  langItemButton: '.lang-item[role="option"]',

  // Theme
  themeToggle: '.theme-toggle',

  // Window buttons (titlebar)
  winBtn: '.win-btn',
  winBtnClose: '.win-btn-close',
} as const;

/**
 * Dashboard selectors. The dashboard is reachable in E2E only through the env
 * hook that starts it in the dashboard state (see README / fixtures/app.ts
 * `dashboardMode`).
 *
 * Two segmented controls on the dashboard: section-nav (Operations/Audit) and
 * the target switcher (User/Domain/Admin) — both use `[role="radio"]`, so they
 * are disambiguated by their parent container.
 */
export const DASH = {
  // Dashboard root
  root: '.dashboard',
  body: '.dashboard-body',

  // Context-bar (under the titlebar): session badge, base_url, language, theme, logout.
  contextBar: '.context-bar',
  contextBarLeft: '.context-bar-left',
  badge: '.context-bar .badge',
  dot: '.context-bar .dot',
  baseUrl: '.context-bar .muted.mono',
  // Theme-toggle in the context bar — a button with aria-label=theme.toggle (icon).
  contextThemeToggle: '.context-bar .btn-icon[aria-label]',
  // Logout — Ghost Small button with a Logout icon (text = tr!logout).
  logoutButton: '.context-bar .btn',

  // Section navigation (Operations/Audit).
  sectionNav: '.section-nav',
  sectionRadio: '.section-nav [role="radio"]',
  sectionActive: '.section-nav .segmented-active',

  // Card grid.
  dashGrid: '.dash-grid',
  cards: '.dash-grid .card',
  spanAll: '.dash-grid .span-all',

  // CSV card (upload + table) — the first card of the grid (direct child).
  csvCard: '.dash-grid > .card:nth-child(1)',
  chooseFileButton: '.dash-grid .btn-secondary',
  dashStat: '.dash-stat',
  dashStatLabel: '.dash-stat-label',

  // Column mapping (op-row with a profile + auto-detect button).
  mappingRow: '.dash-grid > .card:nth-child(1) .op-row',

  // Password generation controls.
  pwControls: '.pw-controls',
  pwControlsRow: '.pw-controls-row',
  pwLengthSlider: '.pw-length-slider',
  pwClassCheckbox: '.pw-class input[type="checkbox"]',
  pwControlsActions: '.pw-controls-actions',

  // Editable table.
  editableTableWrap: '.editable-table-wrap',
  editableTable: '.editable-table',
  editableTableScroll: '.editable-table-scroll',
  inputCell: '.input-cell',
  inputCellInvalid: '.input-cell-invalid',
  rowInvalid: '.row-invalid',
  colDomain: '.col-domain',
  colUsername: '.col-username',
  colPassword: '.col-password',
  colDisplay: '.col-display',
  colQuota: '.col-quota',
  colActions: '.col-actions',
  tdPassword: '.td-password',
  tdActions: '.td-actions',
  // "Add row" button (in the table footer).
  addRowButton: '.editable-table-foot .btn',
  // Per-row password generation.
  genPwButton: '.btn-gen-pw',
  // Row deletion.
  deleteRowButton: '.td-actions .btn-icon',
  // Password strength indicator (warning icon with a tooltip).
  pwStrengthWarn: '.pw-strength-warn',

  // Operations card (target, Create/Edit/Delete/export/diagnostics buttons) —
  // the second direct child of the grid.
  opsCard: '.dash-grid > .card:nth-child(2)',
  // Target switcher — the second segmented control on the dashboard (in the ops card).
  targetRadio: '.dash-grid > .card:nth-child(2) [role="radio"]',
  targetActive: '.dash-grid > .card:nth-child(2) .segmented-active',
  opRunning: '.op-running',
  spinner: '.spinner',
  progressbar: '[role="progressbar"]',

  // Modal dialogs (shared).
  modalBackdrop: '.modal-backdrop',
  dialog: '[role="dialog"][aria-modal="true"]',
  modalTitle: '.modal-title',
  modalBody: '.modal-body',
  modalFooter: '.modal-footer',
  modalIconDanger: '.modal-icon-danger',
  modalIconInfo: '.modal-icon-info',

  // "Audit" section.
  auditCard: '.dash-grid .card',
  auditLocked: '.audit-locked',
  auditTable: '.audit .table, .dash-grid .table',
  auditVerifyButton: '.btn-secondary',
} as const;

/**
 * UI languages (mirror of `Language::all()` in Rust). Order matches the UI.
 * code is the BCP-47 tag (the attribute/key is not directly accessible in the
 * DOM, but the endonym labels are stable).
 */
export const LANGUAGES = [
  { code: 'en', label: 'English', flag: '🇬🇧' },
  { code: 'de', label: 'Deutsch', flag: '🇩🇪' },
  { code: 'fr', label: 'Français', flag: '🇫🇷' },
  { code: 'es', label: 'Español', flag: '🇪🇸' },
  { code: 'it', label: 'Italiano', flag: '🇮🇹' },
  { code: 'pt', label: 'Português', flag: '🇵🇹' },
  { code: 'nl', label: 'Nederlands', flag: '🇳🇱' },
  { code: 'pl', label: 'Polski', flag: '🇵🇱' },
  { code: 'uk', label: 'Українська', flag: '🇺🇦' },
] as const;

/**
 * Localized strings for verification (exact values from app.<lang>.yml).
 *
 * `openForm` — the login-screen button text (`login.open_form`) — a stable
 * visible marker for i18n tests (independent of the dashboard/session).
 * `langLabel` — the aria-label of the language trigger (`lang.label`) —
 * reflects the current locale even on the login screen.
 */
export const I18N_MARKERS = {
  en: { openForm: 'Open sign-in form', langLabel: 'Language' },
  de: { openForm: 'Anmeldeformular öffnen', langLabel: 'Sprache' },
  fr: { openForm: 'Ouvrir le formulaire de connexion', langLabel: 'Langue' },
  es: { openForm: 'Abrir formulario de inicio de sesión', langLabel: 'Idioma' },
  it: { openForm: 'Apri modulo di accesso', langLabel: 'Lingua' },
  pt: { openForm: 'Abrir formulário de início de sessão', langLabel: 'Idioma' },
  nl: { openForm: 'Aanmeldformulier openen', langLabel: 'Taal' },
  pl: { openForm: 'Otwórz formularz logowania', langLabel: 'Język' },
  uk: { openForm: 'Відкрити форму входу', langLabel: 'Мова' },
} as const;

/**
 * Dashboard localization markers (for dashboard-i18n.spec.ts).
 * Exact values from app.<lang>.yml for keys visible on the dashboard.
 */
export const DASH_I18N: Record<
  string,
  {
    navOps: string;
    navAudit: string;
    targetUser: string;
    actionCreate: string;
    actionDelete: string;
    actionCancel: string;
    sessionActive: string;
    logout: string;
  }
> = {
  en: { navOps: 'Operations', navAudit: 'Audit', targetUser: 'Users', actionCreate: 'Create', actionDelete: 'Delete', actionCancel: 'Cancel', sessionActive: 'Session active', logout: 'Log out' },
  de: { navOps: 'Vorgänge', navAudit: 'Audit', targetUser: 'Benutzer', actionCreate: 'Anlegen', actionDelete: 'Löschen', actionCancel: 'Abbrechen', sessionActive: 'Sitzung aktiv', logout: 'Abmelden' },
  fr: { navOps: 'Opérations', navAudit: 'Audit', targetUser: 'Utilisateurs', actionCreate: 'Créer', actionDelete: 'Supprimer', actionCancel: 'Annuler', sessionActive: 'Session active', logout: 'Déconnexion' },
  es: { navOps: 'Operaciones', navAudit: 'Auditoría', targetUser: 'Usuarios', actionCreate: 'Crear', actionDelete: 'Eliminar', actionCancel: 'Cancelar', sessionActive: 'Sesión activa', logout: 'Cerrar sesión' },
  it: { navOps: 'Operazioni', navAudit: 'Audit', targetUser: 'Utenti', actionCreate: 'Crea', actionDelete: 'Elimina', actionCancel: 'Annulla', sessionActive: 'Sessione attiva', logout: 'Esci' },
  pt: { navOps: 'Operações', navAudit: 'Auditoria', targetUser: 'Utilizadores', actionCreate: 'Criar', actionDelete: 'Eliminar', actionCancel: 'Cancelar', sessionActive: 'Sessão ativa', logout: 'Terminar sessão' },
  nl: { navOps: 'Bewerkingen', navAudit: 'Audit', targetUser: 'Gebruikers', actionCreate: 'Aanmaken', actionDelete: 'Verwijderen', actionCancel: 'Annuleren', sessionActive: 'Sessie actief', logout: 'Afmelden' },
  pl: { navOps: 'Operacje', navAudit: 'Audyt', targetUser: 'Użytkownicy', actionCreate: 'Utwórz', actionDelete: 'Usuń', actionCancel: 'Anuluj', sessionActive: 'Sesja aktywna', logout: 'Wyloguj' },
  uk: { navOps: 'Операції', navAudit: 'Аудит', targetUser: 'Користувачі', actionCreate: 'Створити', actionDelete: 'Видалити', actionCancel: 'Скасувати', sessionActive: 'Сесію активовано', logout: 'Вийти' },
};
