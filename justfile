set shell := ["bash", "-uc"]

guard:
    source scripts/sandbox-env.sh && bash scripts/sandbox-check.sh

run *arguments:
    source scripts/sandbox-env.sh && cargo run -p gitdeck-cli -- . {{arguments}}

probe steps_file *arguments:
    source scripts/sandbox-env.sh && cargo run -q -p gitdeck-suite --bin gitdeck-tty -- --steps-file {{steps_file}} {{arguments}}

ensure-fixtures:
    source scripts/sandbox-env.sh && cargo test -p gitdeck-suite --test suite

check:
    source scripts/sandbox-env.sh && cargo check --workspace --all-targets

fmt:
    source scripts/sandbox-env.sh && cargo fmt --all

lint:
    source scripts/sandbox-env.sh && cargo clippy --workspace --all-targets -- -D warnings

test-unit:
    source scripts/sandbox-env.sh && cargo test --workspace --lib

test-suite:
    source scripts/sandbox-env.sh && export GIT_AUTHOR_NAME=gitdeck-ci GIT_AUTHOR_EMAIL=ci@gitdeck.invalid GIT_COMMITTER_NAME=gitdeck-ci GIT_COMMITTER_EMAIL=ci@gitdeck.invalid && rm -rf artifacts/screenshots && mkdir -p .tmp artifacts && touch .tmp/suite-run-stamp && cargo test --workspace --test suite -- --nocapture

test-e2e:
    source scripts/sandbox-env.sh && export GIT_AUTHOR_NAME=gitdeck-ci GIT_AUTHOR_EMAIL=ci@gitdeck.invalid GIT_COMMITTER_NAME=gitdeck-ci GIT_COMMITTER_EMAIL=ci@gitdeck.invalid && cargo test -p gitdeck-cli --test e2e -- --test-threads=${E2E_THREADS:-4} --nocapture

screenshots:
    source scripts/sandbox-env.sh && bash scripts/collect-screenshots.sh

snapshots:
    source scripts/sandbox-env.sh && INSTA_UPDATE=always cargo insta test --workspace --accept

verify: guard fmt lint test-unit test-suite test-e2e screenshots

clean-verify:
    bash scripts/clean-clone-verify.sh
