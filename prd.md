### Product Requirements Document – *“n8n-git” CLI*

---

#### 1. Purpose

Give developers a friction-free way to **pull, edit, version-control, and push n8n workflows** from the terminal, using Git locally and the n8n REST API remotely. ([docs.n8n.io][1], [docs.n8n.io][2])

---

#### 2. Goals & Success Metrics

| Goal                        | KPI / Target                                                                                  |
| --------------------------- | --------------------------------------------------------------------------------------------- |
| Zero-config after first run | > 95 % of subsequent invocations skip setup prompt                                            |
| Round-trip workflow update  | `pull → edit → push` in < 10 s for workflows ≤ 250 kB                                         |
| Security                    | API key never written to plain-text files; passes OWASP ASVS 4.0 L1 secrets storage checklist |
| Reliability                 | < 0.5 % CLI exits with un-handled error across 1 000 executions (telemetry opt-in)            |

---

#### 3. Personas

* **Automation Developer** – comfortable with Git & CLI, wants workflow history and code-review.
* **Ops Engineer** – manages self-hosted n8n, needs reproducible deployments and CI integration.

---

#### 4. User Stories (high-level)

1. *First-time setup* – As a dev, I want the CLI to ask for my n8n URL and API key, store them safely, and never ask again unless I run `n8n-git config`.
2. *Create blank workflow* – I can run `n8n-git new "My Flow"` to create a remote workflow, clone it locally into a new Git repo, and open the JSON in my editor.
3. *List & select* – I can type `n8n-git list` to see my workflows, arrow-select one, and automatically pull it.
4. *Edit & Push* – After editing the JSON (or committing changes), `n8n-git push` uploads and overwrites the remote workflow version.
5. *CI mode* – I can run `n8n-git pull --id 123` inside CI to download the latest revision without interactive prompts.

---

#### 5. Functional Requirements

| ID  | Requirement                                                                                                                                          |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| F-1 | CLI detects absence of `~/.config/n8n-git/config.toml`; launches guided setup.                                                                       |
| F-2 | Store **endpoint** in `confy` config file, **API key** in OS keychain via `keyring`.                                                                 |
| F-3 | Command hierarchy (`clap`): `new`, `pull`, `push`, `list`, `config`, `upgrade`.                                                                      |
| F-4 | `new` calls `POST /workflows` with minimal body → receives ID → downloads JSON and scaffolds `workflows/<slug>` folder, runs `git init` and commits. |
| F-5 | `pull` fetches JSON via `GET /workflows/{id}`, writes to `<slug>/workflow.json`, commits if changed.                                                 |
| F-6 | `push` reads local JSON, sends `PUT /workflows/{id}`; on HTTP 409 offers `--force`.                                                                  |
| F-7 | Interactive selection uses fuzzy list if STDOUT is TTY; falls back to flags in non-interactive environments.                                         |
| F-8 | Verbose flag prints progress bars; quiet flag suppresses all but errors.                                                                             |
| F-9 | Gracefully handles 401/403 (wrong key), 404 (deleted workflow), network timeouts, and Git merge conflicts.                                           |

---

#### 6. Non-Functional Requirements

* **Security** – Secrets only live in memory & OS credential store; TLS enforced; optional `--insecure` for self-signed labs.
* **Portability** – Works on Linux, macOS, Windows; cross-compiled releases via GitHub Actions.
* **Performance** – Single workflow round-trip ≤ 500 ms per 100 kB payload on LAN.
* **Extensibility** – Modular command design, feature-flagged for future sub-commands (e.g. execute workflow).
* **Observability** – Structured logs with `tracing`; optional anonymous telemetry event “command finish + exit code”.

---

#### 7. CLI Flow (happy path)

```text
$ n8n-git           # first run
? n8n API endpoint:  https://automation.mycorp.com/api/v1
? API Key: ********
✔ Credentials saved to system keychain
✔ Config written to ~/.config/n8n-git/config.toml
? What next?  ▸  Create new workflow
               ▹  Pull existing workflow
```

---

#### 8. Folder & Git Layout

```
~/n8n-workflows/
└── my-flow/
    ├── workflow.json       # canonical source of truth
    ├── README.md           # optional docs
    └── .git/               # git2-initialised repo
```

Each `pull` makes a commit:
`feat: sync from n8n (#<workflow-id>)`.

---

#### 9. Recommended Rust Crates

| Concern                   | Crate                                                                                                      | Why / Notes |
| ------------------------- | ---------------------------------------------------------------------------------------------------------- | ----------- |
| HTTP client               | `reqwest 0.12+` – async + native-TLS, timeout & proxy support ([crates.io][3])                             |             |
| Async runtime             | `tokio 1.44` – de-facto standard; recent broadcast-channel fix ([github.com][4])                           |             |
| CLI parsing & completions | `clap 5.x` – derive macros, `clap_complete` for shell autocompletion ([github.com][5])                     |             |
| Config file               | `confy` – zero-boilerplate TOML + directories-aware paths ([github.com][6])                                |             |
| Secret storage            | `keyring 4.x` – cross-platform bindings to macOS Keychain, Windows DPAPI, Linux libsecret ([crates.io][7]) |             |
| Password prompt fallback  | `rpassword` – no-echo terminal read on all OSes ([crates.io][8])                                           |             |
| Interactive menus         | `dialoguer` or `inquire` – fuzzy-select, multi-select, validate (no citation needed; mature crates)        |             |
| JSON (de)serialise        | `serde` + `serde_json` – ubiquity & zero-copy where possible                                               |             |
| Git ops                   | `git2 0.20` – safe Rust bindings to libgit2 ≥ 1.9.0 ([github.com][9], [github.com][10])                    |             |
| Progress bars             | `indicatif` – multi-bar rendering, useful for uploads                                                      |             |
| Error handling            | `anyhow` for top-level `Result`, `thiserror` for domain errors                                             |             |
| Path helpers              | `directories` – resolve config/cache/data dirs per-OS                                                      |             |

> **Why keyring + confy together?**
> `confy` persists non-secret preferences (endpoint, defaults), while `keyring` writes the API key into the OS credential store, keeping plaintext out of dotfiles.

---

#### 10. Milestones & Timeline (MVP)

| Week | Deliverable                                                          |
| ---- | -------------------------------------------------------------------- |
| 1    | Project scaffold, `setup` command, config & key storage              |
| 2    | `new` & `pull` commands with Git bootstrap                           |
| 3    | `push` with conflict handling + unit tests (mockito)                 |
| 4    | Interactive menus, coloured output, binary release CI                |
| 5    | Beta testing, telemetry opt-in, docs, `homebrew` & `winget` formulas |

---

#### 11. Risks & Mitigations

| Risk                      | Impact              | Mitigation                                                              |
| ------------------------- | ------------------- | ----------------------------------------------------------------------- |
| n8n API schema changes    | Push/pull may break | Pin API version (`/v1`), graceful JSON fallback                         |
| Libgit2 breaking change   | Git ops fail        | Vendor tested libgit2 via `git2` default feature; lockfile CI           |
| Secrets leakage via debug | Accidental print    | Scrub API key in logs; require `RUST_LOG=trace` to print request bodies |

---

#### 12. Future Extensions

* **Diff & merge** – apply structural JSON diff before commit.
* **Template generators** – bootstrap common flows from community templates.
* **CI plugin** – GitHub Action wrapping `n8n-git` for automated deployment.
* **Remote execution** – trigger workflow run after successful push.

---

### Next Steps

1. Validate crate choices with a spike prototype.
2. Lock API contract & edge cases with the n8n team.
3. Start week-1 scaffold.

Let me know if you’d like wire-commands, code snippets, or a deeper dive into any section!

[1]: https://docs.n8n.io/api/?utm_source=chatgpt.com "n8n public REST API Documentation and Guides - n8n Docs"
[2]: https://docs.n8n.io/api/api-reference/?utm_source=chatgpt.com "API reference - n8n Docs"
[3]: https://crates.io/crates/reqwest?utm_source=chatgpt.com "reqwest - crates.io: Rust Package Registry"
[4]: https://github.com/tokio-rs/tokio/blob/master/tokio/CHANGELOG.md?utm_source=chatgpt.com "tokio/tokio/CHANGELOG.md at master - GitHub"
[5]: https://github.com/clap-rs/clap?utm_source=chatgpt.com "clap-rs/clap: A full featured, fast Command Line Argument ... - GitHub"
[6]: https://github.com/rust-cli/confy?utm_source=chatgpt.com "rust-cli/confy: Zero-boilerplate configuration management in ... - GitHub"
[7]: https://crates.io/crates/keyring?utm_source=chatgpt.com "keyring - Rust Package Registry - Crates.io"
[8]: https://crates.io/crates/rpassword?utm_source=chatgpt.com "rpassword - crates.io: Rust Package Registry"
[9]: https://github.com/rust-lang/git2-rs?utm_source=chatgpt.com "rust-lang/git2-rs: libgit2 bindings for Rust - GitHub"
[10]: https://github.com/rust-lang/git2-rs/blob/master/CHANGELOG.md?utm_source=chatgpt.com "CHANGELOG.md - rust-lang/git2-rs - GitHub"
