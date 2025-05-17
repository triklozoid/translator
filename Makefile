.PHONY: all build test test-seq test-coverage test-coverage-html run clean install lint fmt check release

# Default action
all: build

# Build the project
build:
	cargo build

# Build in release mode
release:
	cargo build --release

# Run the application
run:
	cargo run

# Run all tests (may fail due to parallel execution)
test:
	cargo test

# Run tests sequentially (recommended for this project)
test-seq:
	cargo test -- --test-threads=1

# Run tests with coverage measurement
test-coverage:
	cargo llvm-cov test -- --test-threads=1

# Generate HTML coverage report
test-coverage-html:
	cargo llvm-cov --html
	@echo "Coverage report generated in target/llvm-cov/html/index.html"

# Open coverage report in browser
test-coverage-open:
	cargo llvm-cov --open

# Install the application
install:
	cargo install --path .

# Lint the code
lint:
	cargo clippy -- -D warnings

# Format the code
fmt:
	cargo fmt

# Check formatting without making changes
fmt-check:
	cargo fmt -- --check

# Run all checks (format, lint, tests)
check: fmt-check lint test-seq

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/llvm-cov

# Install development dependencies
dev-setup:
	rustup component add clippy rustfmt llvm-tools-preview
	cargo install cargo-llvm-cov

# Run specific test file
test-config:
	cargo test --test config_tests -- --test-threads=1

test-settings:
	cargo test --test settings_tests -- --test-threads=1

test-translation:
	cargo test --test translation_unit_tests -- --test-threads=1

test-ui:
	cargo test --test ui_tests -- --test-threads=1

test-integration:
	cargo test --test integration_tests -- --test-threads=1

# Help
help:
	@echo "Available targets:"
	@echo "  all              - Build the project (default)"
	@echo "  build            - Build the project in debug mode"
	@echo "  release          - Build the project in release mode"
	@echo "  run              - Run the application"
	@echo "  test             - Run all tests"
	@echo "  test-seq         - Run tests sequentially (recommended)"
	@echo "  test-coverage    - Run tests with coverage measurement"
	@echo "  test-coverage-html - Generate HTML coverage report"
	@echo "  test-coverage-open - Open coverage report in browser"
	@echo "  install          - Install the application"
	@echo "  lint             - Run clippy linter"
	@echo "  fmt              - Format the code"
	@echo "  fmt-check        - Check code formatting"
	@echo "  check            - Run all checks (format, lint, tests)"
	@echo "  clean            - Clean build artifacts"
	@echo "  dev-setup        - Install development dependencies"
	@echo "  test-config      - Run config tests only"
	@echo "  test-settings    - Run settings tests only"
	@echo "  test-translation - Run translation tests only"
	@echo "  test-ui          - Run UI tests only"
	@echo "  test-integration - Run integration tests only"
	@echo "  help             - Show this help message"