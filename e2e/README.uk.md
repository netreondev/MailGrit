# MailGrit — E2E (Playwright + CDP)

Наскрізні (end-to-end) тести десктоп-застосунку MailGrit. Підключення до
зібраного `.exe` (Dioxus -> WebView2) відбувається через **Chrome DevTools
Protocol** — без окремого браузера; тестується реальний UI у реальному WebView2.

> Ізольовано від Rust-збірки: не торкається workspace і не вводить Node/JS-
> інструментарій у збірку. Усі файли — лише в цій теці.

**English:** [README.md](README.md)

## Передумови

- **Лише Windows** — суіт керує реальним `.exe` та його WebView2 через CDP
  (на інших ОС нема WebView2; суіт у такому вигляді не переносний).
- **Node.js >= 20** (на машині розробника; у Rust-збірці не потрібен)
- **WebView2 Runtime** (у Windows 11 передстановлений)
- Зібраний `.exe` **із E2E-хуками**: `cargo build -p mailgrit-app-desktop --features e2e`
  (debug достатньо). Саме cargo-фича `e2e` вмикає `e2e_state.rs` та перевизначення
  `MAILGRIT_LOGIN_URL` — у release-збірках цього коду нема взагалі.

Суіт запускається в CI на кожен push (джоба `e2e` у `.github/workflows/ci.yml`,
`windows-latest`): збірка з `--features e2e`, `npm ci` і `npx playwright test`.

## Запуск

```bash
cargo build -p mailgrit-app-desktop --features e2e   # з кореня репозиторія
cd e2e
npm ci
npm run typecheck                 # tsc --noEmit (Playwright ігнорує типові помилки)
npm run test:jsmoke               # виконує реальний згенерований batch-JS (mocked fetch)
npm test                          # усі тести (підключення до власного WebView2 застосунку)
npm run test:headed               # з видимим вікном (зручно під час дебагу)
npm run report                    # HTML-звіт останнього прогону
```

Завантажувати браузери Playwright не потрібно: `chromium.connectOverCDP`
підключається до власного WebView2 застосунку — НЕ запускайте
`npx playwright install chromium` (він завантажує браузер, який суіт не використовує).
stdout/stderr застосунку кожного прогону пишуться в `app-stdio.log`
у тимчасовому каталозі прогону; упалений тест зберігає цей лог у вивід
Playwright (`test-results/…/app-stdio.log`) перед очищенням — діагностика
застосунку переживає падіння.

## Як це працює

1. `fixtures/app.ts` знаходить `.exe` і **копіює його до тимчасового каталогу**
   (ізолюючи `mailgrit-data/` — кожен прогон стартує з чистим config/cookie-store).
   Перед spawn також перевіряється, що CDP-порт ВІЛЬНИЙ — завислий
   `msedgewebview2.exe`, який тримає порт, завершує прогон одразу з названою
   причиною замість вводячих в оману помилок сторінок.
2. Перед spawn інжектується
   `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<CDP_PORT>`
   (порт за замовчуванням визначений один раз у `playwright.config.ts`; CI
   передає те саме значення через змінну середовища `CDP_PORT`).
3. Після старту процесу чекаємо на CDP endpoint ->
   `chromium.connectOverCDP(...)` -> беремо сторінку вікна MailGrit -> передаємо її в тест як `{ page }`.
4. Teardown завершує дерево процесів (await-нений `taskkill /T /F`) і видаляє
   тимчасовий каталог.

> **НЕ копіюйте HKLM-політику з CI на робочу машину.** CI встановлює
> `HKLM\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments\*`,
> бо підвищені (elevated) раннери ігнорують змінні `WEBVIEW2_*` (Runtime 150+).
> Ця політика застосовується до **КОЖНОГО WebView2-застосунку на хості** і
> відкриває remote-debugging (CDP) порт у всіх них — будь-який локальний
> процес зможе керувати тими браузерами. Це прийнятно лише на ефемерному раннері.

`workers: 1` (один `.exe` + один CDP-порт одночасно).

## Набір тестів

### Екран входу (звичайний старт)

| Файл | Що перевіряє |
|------|--------------|
| `launch.spec.ts` | старт, titlebar, екран входу, заголовок вікна |
| `branding.spec.ts` | SVG-логотип («Forged Spark»), градієнт, текст «MailGrit» |
| `language.spec.ts` | **регресія** багу нерозкривного селектора мови |
| `i18n.spec.ts` | цикл 9 мовами, зміна локалізованого тексту |
| `theme.spec.ts` | dark/light, персистентність у config.toml |
| `url-validation.spec.ts` | валідація URL сервера (реальна UI-логіка) |
| `window-controls.spec.ts` | кнопки minimize/maximize |

### Дашборд (старт через env-хук `MAILGRIT_E2E_DASHBOARD`)

Ці spec-файли імпортують `testDashboard` із `fixtures/app` — застосунок стартує
одразу в стані дашборду з попередньо заповненими тестовими рядками (минути
login-flow iRedAdmin). Фокус — оцінка **якості, симетрії, працездатності та
зрозумілості** всіх екранів без мережевого round-trip.

| Файл | Що перевіряє |
|------|--------------|
| `dashboard.spec.ts` | доступ до дашборду, навігація розділами, ціль, logout |
| `dashboard-layout.spec.ts` | **симетрія**: центрування сітки, без перекриттів, colgroup |
| `dashboard-theme.spec.ts` | **тема/контраст**: токени dark/light, WCAG AA >= 4.5, surface != bg |
| `modals.spec.ts` | модалки (delete/regenerate/master-password): ARIA, центрування, закриття |
| `editable-table.spec.ts` | таблиця: add/edit/delete, валідація (підсвічування), per-row пароль |
| `password-controls.spec.ts` | слайдер довжини, policy-locked чекбокси, fill-empty, regenerate-all |
| `dashboard-i18n.spec.ts` | локалізація всіх екранів 9 мовами, без битих ключів |
| `a11y.spec.ts` | **доступність**: ARIA-ролі, доступні імена, focus-ring, h2-семантика |

Утиліти оцінки якості — `helpers/layout.ts` (`assertContrast`, `assertCenteredBoth`,
`assertSymmetricMargins`, `parseColor`, `contrastRatio`, `assertNoRawKey`).

## E2E-хук старту в дашборді (`MAILGRIT_E2E_DASHBOARD`)

Дашборд у реальному застосунку доступний лише через авто-виявлення входу
login-webview'ом (навігація на `/dashboard`), а Dioxus `Signal<AppState>`
недоступний із CDP/JS напряму. Тому для E2E-покриття дашборду без живого сервера
реалізовано тестовий хук у Rust (`crates/app-desktop/src/e2e_state.rs`):

- Активується **лише** env-змінною `MAILGRIT_E2E_DASHBOARD=1`.
- Без env (production) — повний no-op: застосунок стартує на екрані входу, як
  зазвичай. Хук не впливає на release-збірку.
- Під час активації парсить вбудований валідний CSV (2 рядки) через той самий
  канонічний парсер (`parse_csv_bytes_auto`), що й завантаження користувача, і
  встановлює `screen=Dashboard`, `session_ok=true`, `auth_status=Connected`,
  попередньо заповнює `editable_rows`/`csv`/`column_mapping`.
- Застосовується один раз при старті (`use_hook`), а не на кожен рендер — інакше
  logout (скидання `screen=Login`) одразу відкочувався б назад.

Фікстура `testDashboard` у `fixtures/app.ts` встановлює цей env перед spawn.

## Що НЕ тестується

- Реальний вхід в iRedAdmin (потрібен живий сервер) — поза областю. Дашборд
  тестується через env-хук старту (див. вище), а не через справжній вхід.
- Завантаження CSV через нативний `rfd`-діалог (недоступний Playwright). Натомість
  таблиця попередньо заповнюється env-хуком; парсер CSV покритий Rust
  unit/property/fuzz-тестами.
- Мережевий round-trip масових операцій (create/edit/delete проти iRedAdmin).
  Операції йдуть JS `fetch` усередині окремого login-webview; контракт маркерів
  успіху/помилки покритий Rust-тестами (`webview_markers_tests.rs`).
