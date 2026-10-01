# Contributing

The restoration aims to preserve the original game's behavior, story and visuals.
Use the original project data and documented reference behavior when changing
compatibility code. Describe the observed problem and the evidence for a fix.

## Development setup

Follow [Getting started](README.md#getting-started). Runtime assets are tracked;
`original/`, `reference/`, build outputs and local saves are excluded. The asset
converter is only needed when changing legacy parsing or converted data.

Before submitting a change:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build -p amnezia --release --locked
python3 scripts/verify-playtest-evidence.py
```

The original-vocabulary audit requires the untracked `original/RPG_RT.ldb` and
is explicitly ignored in the default suite. When the original extraction is
available, run it separately:

```sh
cargo test -p amnezia-convert --locked every_original_term_survives_conversion_including_intentional_blanks -- --ignored
```

For gameplay changes, add a regression that checks the affected behavior and
run the relevant native/offscreen scenario from [Testing](docs/TESTING.md).
Record what was actually observed and which cases remain unverified. A fixture
or edited save must be identified as such.

Keep legacy parser, converter, runtime data and execution behavior consistent.
Use `--data-only` when regenerating structured assets without changing media.
Do not commit `target/`, the original game extraction, local credentials or
unrelated personal files. Changes to story routes and release acceptance should
update [Compatibility status](docs/STATUS.md).

## Gameplay bug reports

Include the build/commit, operating system and CPU architecture, selected story
branch, steps from a known save, and expected versus observed behavior. Attach a
screenshot and a copy of the relevant save when possible. Preserve your original
save before experimenting. Explain any level, skill, item or EP edits.

The project is a non-commercial restoration. Original game assets and third-party
materials retain their owners' rights; see [Credits](README.md#credits-and-third-party-materials).
