# Orchestrator — Makefile
#
# The one entry point for common tasks. No one memorises cargo flags.

.DEFAULT_GOAL := help
.PHONY: help build release test test-system lint fmt docs deps gui dev-stack check clean \
        design-spec

SHELL := /bin/bash
CARGO ?= cargo
NPM ?= npm

help: ## Show this help
	@grep -hE '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

build: ## Build the whole workspace
	$(CARGO) build --workspace

release: ## Build release binaries and the GUI bundle
	$(CARGO) build --workspace --release
	npm --prefix orchestrator-gui run build

test: ## Run the unit and integration tests
	$(CARGO) test --workspace

test-system: ## Run the privileged Linux system tests (needs root)
	./system-tests/run.sh

lint: ## Run every linter
	$(CARGO) fmt --all --check
	$(CARGO) clippy --workspace --all-targets -- -D warnings
	$(CARGO) deny check 2>/dev/null || echo "cargo-deny is not installed — skipping"
	npm --prefix orchestrator-gui run lint

fmt: ## Format the code
	$(CARGO) fmt --all
	npm --prefix orchestrator-gui run format

docs: ## Build the Rust documentation
	$(CARGO) doc --workspace --no-deps

deps: ## Show outdated dependencies
	$(CARGO) outdated
	npm --prefix orchestrator-gui outdated || true

gui: ## Start the GUI dev server
	npm --prefix orchestrator-gui run dev

dev-stack: ## Start a local multi-tenant stack
	./scripts/bootstrap.sh

design-spec: ## Regenerate plan/DESIGN_SPEC.md from the numbered documents
	./scripts/build-design-spec.sh

check: lint test build ## Everything CI runs

clean: ## Remove build artifacts
	$(CARGO) clean
	rm -rf orchestrator-gui/dist
