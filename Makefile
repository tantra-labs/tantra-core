.PHONY: default build build-dev build-prod build-release build-debug dev prod release debug check check-dev check-prod test clippy fmt clean schema schema-check schema-freeze

# Default target is 'prod' (no debug build unless asked for specifically)
default: prod
build: prod

prod: build-prod
build-prod:
	cargo build --workspace --profile prod

release: build-release
build-release:
	cargo build --workspace --release

dev: build-dev
build-dev:
	cargo build --workspace --profile dev

# Debug build is explicit and not run unless asked
debug: build-debug
build-debug:
	cargo build --workspace

check: check-prod
check-prod:
	cargo check --workspace --profile prod

check-dev:
	cargo check --workspace --profile dev

test:
	cargo test --workspace --profile prod

clippy:
	cargo clippy --workspace --all-targets --profile prod -- -D warnings

fmt:
	cargo fmt

# Regenerate schema/domain/domain_schema.json from the Rust derives
schema:
	TANTRA_SCHEMA_BLESS=1 cargo test --profile prod --test schema_compat_recursive domain_schema_is_regenerated

# Verify the generated schema is fresh and backward compatible
schema-check:
	cargo test --profile prod --test schema_compat_recursive
	git diff --exit-code -- schema/domain/domain_schema.json

# Freeze the current SCHEMA_VERSION snapshot (run after every version bump)
schema-freeze:
	@N=$$(grep -oP 'pub const SCHEMA_VERSION: u32 = \K[0-9]+' src/lib.rs); \
	$(MAKE) schema; \
	if [ -e schema/compatibility/v$${N}_domain_schema.json ]; then \
		echo "schema/compatibility/v$${N}_domain_schema.json already frozen"; exit 1; \
	fi; \
	cp schema/domain/domain_schema.json schema/compatibility/v$${N}_domain_schema.json; \
	mkdir -p schema/golden/v$${N}

clean:
	cargo clean
	rm -rf target
