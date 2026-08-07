# MailGrit

[![CI](https://github.com/netreondev/MailGrit/actions/workflows/ci.yml/badge.svg)](https://github.com/netreondev/MailGrit/actions/workflows/ci.yml)
[![Ліцензія: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#ліцензія)
[![Rust 1.97.1](https://img.shields.io/badge/rust-1.97.1-orange.svg)](https://www.rust-lang.org)
[![Платформи](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS%20ARM-lightgrey.svg)](#платформи)

> Кросплатформний десктоп-клієнт (Windows / Linux / macOS ARM) для масової
> автоматизації **iRedAdmin**. Побудований на **Dioxus desktop** (без
> Node/JS-інструментарію), зі вбудованим браузером для авторизації та нативною
> панеллю масових операцій.

**English:** [README.md](README.md) · **Вебсайт:** [netreondev.github.io/MailGrit](https://netreondev.github.io/MailGrit)

---

## Швидкий старт

```bash
cargo build --release -p mailgrit-app-desktop
# → target/release/mailgrit-app-desktop[.exe]
```

Під час першого запуску поруч із бінарником створюється тека `mailgrit-data/`,
де зберігаються всі файли застосунку.

## Використання

> ⚠️ **Лише авторизоване використання.** MailGrit виконує масове
> створення/редагування/видалення на сервері iRedAdmin. Використовуйте його
> **лише на системах і акаунтах, якими ви володієте або які авторизовані
> адмініструвати.** Неавторизоване використання є незаконним і є вашою
> виключною відповідальністю. Див. [DISCLAIMER.uk.md](DISCLAIMER.uk.md).

1. Запустіть `mailgrit-app-desktop`. Введіть адресу iRedAdmin
   (наприклад, `https://mail.example.com/iredadmin`) і натисніть **Відкрити
   форму входу**. Відкриється вікно застосунку зі справжньою формою iRedAdmin.
2. Увійдіть в iRedAdmin — **більше нічого натискати не треба**: застосунок
   автоматично виявляє вхід (гібридний предикат — див. *Як працює авторизація*)
   і сам переходить на панель операцій.
3. Завантажте CSV (`domain,username,password,display_name,quota_mb`).
4. Виконайте масове **створення / редагування / видалення**.

### Режим роботи

Підтримується лише один режим — **OSE (форми)**: масові операції виконуються як
JS `fetch` POST стандартної HTML-форми створення/редагування iRedAdmin із CSRF-
токеном. Поля форми:
`csrf_token, domainName, username, newpw, confirmpw, cn, preferredLanguage,
mailQuota, submit_add_user`.

> Запити йдуть через вбудований браузер webview із легітимною сесією, тож режим
> працює й за FortiWeb/WAF.

### Інтерфейс

Безрамковий UI: власний titlebar з брендом та кнопками вікна, темна/світла теми
з перемикачем (зберігається в `config.toml`), симетрична сітка карток, модальне
підтвердження деструктивних операцій та індикатори прогресу. Усі компоненти —
власна дизайн-система на CSS-токенах (без Node/JS-інструментарію).

### Діагностика

Кнопка **Діагностика форм** виконує GET сторінки створення користувача та
повертає HTML форми (імена полів, URL action, CSRF), показуючи його в UI та логу.

**Повний дамп операцій** фіксує для кожної операції: URL/метод/Content-Type
запиту, усі поля форми (із маскуванням пароля/CSRF/email/PII), значення CSRF і
статус GET-форми, статус/заголовки HTTP-відповіді, повне тіло відповіді сервера
(до 5000 символів), а також маркери успіху/помилки та результат пост-перевірки.

## Як працює авторизація

Вхід виявляється **data-driven** — за навігацією login-webview на `/dashboard`, а
не вгадуванням імені куки. iRedAdmin, Django та FortiWeb використовують різні
імена кук, тож вгадування крихке; за WAF бекенд-сесію утримує проксі, і повтор
куки в окремому HTTP-клієнті не автентифікує бекенд. Тому операції виконуються як
JS `fetch()` **усередині того ж webview**, що тримає легітимну сесію.

## Модель безпеки

- **Шифрований ланцюговий аудит-лог** — кожна операція додається до ланцюга з
  виявленням втручання (HMAC-SHA256), шифрується at-rest потоковим AEAD
  (XChaCha20-Poly1305).
- **Майстер-пароль** — виводить ключ аудит-лога та ключ шифрування експорту через
  Argon2id (memory-hard KDF). Пароль ніколи не зберігається; якщо його втрачено,
  аудит-лог і шифровані експорти розблокувати неможливо.
- **`unsafe_code = "forbid"`** на рівні workspace — жодного `unsafe` у застосунку
  (wry 0.53 віддає HttpOnly-куки нативно).

## Формат CSV

```
domain,username,password,display_name,quota_mb
example.com,john,S3cret!,John Doe,512
```

- BOM знімається автоматично; кодування — UTF-8.
- Гнучке зіставлення колонок: імена заголовків зіставляються з канонічними полями
  без урахування регістру, тож локалізовані або перейменовані заголовки все одно
  працюють.
- Жорсткі ліміти (рядки, довжина рядка, довжина поля) захищають від випадкових
  великих входів; див. константи лімітів у `core-domain`.

## Інтернаціоналізація (i18n)

UI постачається **9 мовами**:

| Код | Мова        | Самоназва   |
|-----|-------------|-------------|
| en  | Англійська  | English     |
| de  | Німецька    | Deutsch     |
| fr  | Французька  | Français    |
| es  | Іспанська   | Español     |
| it  | Італійська  | Italiano    |
| pt  | Португальська | Português |
| nl  | Нідерландська | Nederlands |
| pl  | Польська    | Polski      |
| uk  | Українська  | Українська  |

Переклади вбудовуються в бінарник під час компіляції (`rust-i18n`); обрана мова
зберігається в `config.toml`.

## Архітектура

Cargo-workspace з п'яти crate'ів, кожен компілюється/тестується ізольовано
(`cargo test -p <crate>`):

| Crate | Відповідальність |
|-------|------------------|
| `core-domain` | Доменні типи (Newtype/Typestate), доменні помилки, константи лімітів, політика паролів |
| `core-csv` | Парсинг та валідація CSV зі зняттям BOM і жорсткими лімітами пам'яті |
| `core-storage` | Локальне сховище SQLite: журнал операцій, ланцюговий аудит-лог |
| `core-security` | Криптографія at-rest: потоковий AEAD, HMAC-ланцюг, Argon2id KDF |
| `app-desktop` | Десктоп-застосунок Dioxus 0.7: вікно входу, робота з куки, нативна RSX-панель |

```
core-domain ─┬─ core-csv ────┐
             ├─ core-storage ─┤
             └─ core-security ┴─ app-desktop
```

## Розробка

| Завдання | Команда |
|----------|---------|
| Форматування | `cargo fmt --all` (перевірка: `cargo fmt --all -- --check`) |
| Лінтер | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| Тести | `cargo nextest run --workspace` |
| Док-тести | `cargo test --workspace --doc` |
| Release-збірка | `cargo build --release -p mailgrit-app-desktop` |
| Запуск із debug-логом | `$env:RUST_LOG="debug"; .\target\release\mailgrit-app-desktop.exe` |
| Ланцюг постачання | `cargo deny check advisories bans licenses sources`, потім `cargo audit` |
| Невикористані залежності | `cargo machete --skip-target-dir` |
| SemVer | `cargo semver-checks --workspace` |

### Дисципліна лінтів

Workspace застосовує сувору декларативну політику лінтів у `Cargo.toml`:
`panic`, `unwrap_used`, `expect_used`, `indexing_slicing`,
`arithmetic_side_effects`, `todo`/`unimplemented`/`unreachable`, `dbg_macro` та
`print_stdout`/`print_stderr` — усі `deny`; `unsafe_code` — `forbid`. Усі групи
clippy (`correctness`, `suspicious`, `complexity`, `perf`, `pedantic`, `nursery`,
`cargo`) — `deny`. Єдине задокументоване виключення — `doc_markdown = "allow"`
(хибні спрацьовування на технічні абревіатури).

### Інструментарій

Rust **1.97.1**, закріплений через `rust-toolchain.toml` (edition 2024).

## Платформи

| Платформа | Ціль |
|-----------|------|
| Windows | `x86_64-pc-windows-msvc` |
| Linux | `x86_64-unknown-linux-gnu` |
| macOS (Apple Silicon) | `aarch64-apple-darwin` |

CI проганяє повний гейт якості (fmt, clippy, nextest, док-тести, cargo-deny,
cargo-audit, cargo-machete, cargo-semver-checks) на усіх трьох, плюс матриця
release-збірок. Формальна верифікація (Kani, Miri), мутаційне тестування
(cargo-mutants) та безперервний фаззинг (cargo-fuzz) запускаються щодня і не
блокують PR.

## Ліцензія

Двойне ліцензовано під **MIT OR Apache-2.0** на ваш вибір. Див.
[LICENSE-MIT](LICENSE-MIT) та [LICENSE-APACHE](LICENSE-APACHE). Внески робляться
на тих самих умовах подвійної ліцензії (inbound = outbound); див.
[CONTRIBUTING.uk.md](CONTRIBUTING.uk.md).

## Правові питання та конфіденційність

| Документ | Призначення |
|----------|-------------|
| [DISCLAIMER.uk.md](DISCLAIMER.uk.md) | Політика прийнятного використання, відсутність гарантій, обмеження відповідальності |
| [PRIVACY.uk.md](PRIVACY.uk.md) | Обробка даних — лише локальне сховище, маскування PII, без телеметрії |
| [SECURITY.uk.md](SECURITY.uk.md) | Звітність про вразливості та модель безпеки |
| [NOTICE.uk.md](NOTICE.uk.md) | Авторські права, ліцензування, атрибуція товарних знаків |

**Товарні знаки:** "iRedAdmin" є товарним знаком відповідних власників. MailGrit
**не афілійований, не схвалений та не спонсорується** iRedAdmin чи її
розробниками; назва використовується виключно для позначення сумісності. Див.
[NOTICE.uk.md](NOTICE.uk.md).

## Підтримка проєкту

MailGrit безкоштовний і має відкритий код. Якщо він економить ваш час — підтримайте
розвиток проєкту:

- [Донат](https://donatello.to/VladymyrM)

