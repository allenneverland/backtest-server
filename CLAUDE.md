# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

### Code Quality Maintenance

- Refactor continuously to improve code quality
- Fix technical debt early before it compounds
- Leave code cleaner than you found it
- Make small, focused commits with clear commit messages
- Use meaningful branch names that reflect the purpose of changes
- Follow the repository's version control practices

### Version Control Best Practices

- Write clear commit messages that explain the why, not just the what
- Make small, focused commits that do one thing
- Use meaningful branch names that reflect the feature or fix

This project uses asynchronous programming extensively. Follow these guidelines for working with async code:

### Async Runtime and Basics

- Use `tokio` as the primary async runtime for handling asynchronous tasks and I/O
- Implement async functions using `async fn` syntax
- Leverage `tokio::spawn` for task spawning and concurrency
- Use `tokio::select!` for managing multiple async tasks and cancellations
- Favor structured concurrency: prefer scoped tasks and clean cancellation paths
- Implement timeouts, retries, and backoff strategies for robust async operations

### Channels and Concurrency

- Use `tokio::sync::mpsc` for asynchronous, multi-producer, single-consumer channels
- Use `tokio::sync::broadcast` for broadcasting messages to multiple consumers
- Implement `tokio::sync::oneshot` for one-time communication between tasks
- Prefer bounded channels for backpressure; handle capacity limits gracefully
- Use `tokio::sync::Mutex` and `tokio::sync::RwLock` for shared state across tasks, avoiding deadlocks

### Async Error Handling

- Embrace Rust's Result and Option types for error handling
- Use `?` operator to propagate errors in async functions
- Implement custom error types using `thiserror` or `anyhow` for more descriptive errors
- Handle errors and edge cases early, returning errors where appropriate
- Use `.await` responsibly, ensuring safe points for context switching

### Async Testing

- Write unit tests with `tokio::test` for async tests
- Use `tokio::time::pause` for testing time-dependent code without real delays
- Implement integration tests to validate async behavior and concurrency
- Use mocks and fakes for external dependencies in tests

### Async Performance Optimization

- Minimize async overhead; use sync code where async is not needed
- Avoid blocking operations inside async functions; offload to dedicated blocking threads if necessary
- Use `tokio::task::yield_now` to yield control in cooperative multitasking scenarios
- Optimize data structures and algorithms for async use, reducing contention and lock duration
- Use `tokio::time::sleep` and `tokio::time::interval` for efficient time-based operations

## Common Development Commands

### Building and Running

```bash
# Build development version
cargo make build

# Build release version  
cargo make build-release

# Run the server (development)
cargo make run

# Run the server (production)
cargo make run-release

# Watch for changes and auto-build/test
cargo make watch
```

### Testing and Quality

```bash
# Run all tests
cargo make test-all

# Run unit tests
cargo make test

# Run integration tests
cargo make test-integration

# Format code
cargo make format

# Check code formatting
cargo make format-check

# Run linter
cargo make lint

# Generate coverage report
cargo make coverage
```

### Database Management

```bash
# Run database migrations
./scripts/run-migration.sh

# Create a new migration
./scripts/create-migration.sh <migration_name>

# Alternative: Run migrations using Rust binary
cargo run --bin migrate run

# Check migration status
cargo run --bin migrate status
```

### Docker Development

```bash
# Build Docker environment
cargo make docker-build

# Start Docker services
cargo make docker-up

# View Docker status
cargo make docker-ps

# View Docker logs
cargo make docker-logs -f

# Execute commands in Docker
cargo make docker-c <command>

# Enter Docker shell
cargo make docker-exec

# Stop Docker services
cargo make docker-down

# Clean Docker environment
cargo make docker-clean
```

### Running Examples

```bash
# Run a specific example
cargo make run-example <example_name>

# Common examples:
cargo make run-example simple_strategy
cargo make run-example backtest_runner
cargo make run-example messaging_client
```

## High-Level Architecture

### Core Domain Model

The system is built around a financial trading domain with the following key concepts:

1. **Time Series Data**: Generic structure for all time-indexed financial data
2. **Market Data**: OHLCV bars and tick data 
3. **Instruments**: Stocks, futures, options, forex, and crypto
4. **Strategies**: User-defined trading logic using a custom DSL
5. **Backtests**: Historical simulations of strategy performance

### Module Organization

The project uses Rust's modern module system with clear separation of concerns:

```
src/
├── domain_types/     # Core data structures shared across the system
├── data_ingestion/   # Raw data loading and validation
├── data_provider/    # Unified data access layer
├── strategy/         # Strategy management and lifecycle
├── dsl/             # Domain-specific language for strategies
├── runtime/         # Sandboxed strategy execution environment
├── execution/       # Order simulation and position management  
├── backtest/        # Backtesting engine and orchestration
├── messaging/       # RabbitMQ-based communication
├── storage/         # Database and cache layer
├── server/          # HTTP server and API
└── config/          # Configuration management
```

### Key Workflows

1. **Data Pipeline**: Ingestion → Validation → Storage → Provider
2. **Strategy Lifecycle**: Upload → Parse/Compile → Version → Execute
3. **Backtest Flow**: Initialize → Prepare Data → Execute Strategy → Collect Results → Analyze
4. **Message Flow**: External Request → RabbitMQ → Handler → Storage → Response

### External Dependencies

- **Database**: TimescaleDB (PostgreSQL extension) for time-series data
- **Cache**: Redis for fast data access and distributed operations
- **Message Broker**: RabbitMQ for async communication

### Development Patterns

1. **Builder Pattern**: Used extensively for complex object construction (e.g., ServerBuilder)
2. **Repository Pattern**: Data access abstraction in storage module
3. **Strategy Pattern**: For different validator and cleaner implementations
4. **Observer Pattern**: Event bus for loose coupling between components
5. **Sandbox Pattern**: Isolated execution environment for user strategies

## Claude Code Development Guidelines

### Project Awareness & Context

- Always read STRUCTURE.md, PLANNING.md, DEPENDENCIES.md at the start of a new conversation to understand the project's architecture, goals, style, and constraints.
- Check TASK.md before starting a new task. If the task isn't listed, add it with a brief description and today's date.
- Use consistent naming conventions, file structure, and architecture patterns as described in PLANNING.md.
- When exploring a new feature, first identify which module or component it belongs to.
- Check existing implementations before creating new ones to maintain consistency.
- For understanding cross-component interactions, look for integration tests.

### Code Structure & Modularity

- Never create a file longer than 500 lines of code. If a file approaches this limit, refactor by splitting it into modules or helper files.
- Organize code into clearly separated modules, grouped by feature or responsibility.
- Use clear, consistent imports (prefer using the Rust module system properly with `use` statements organized logically).

### Testing & Reliability

- Always create unit tests for new features using `#[cfg(test)]` module with appropriate test functions.
- Always run `cargo check` after editing code to catch compilation errors early.
- After updating any logic, check whether existing unit tests need to be updated. If so, update them.
- Make sure tests are integrated into the CI pipeline to automatically validate changes.
- Write tests before fixing bugs to verify the issue and prevent regressions.
- Keep tests readable and maintainable.
- Test edge cases and error conditions thoroughly.

#### Rust Testing Commands

```bash
# Run all tests in a specific package
cargo nextest run --package <package-name>

# Run documentation tests
cargo test --package <package-name> --doc

# Run tests with specific features
cargo test --package <package-name> --features=<feature1>,<feature2>

# Run tests with all features
cargo test --package <package-name> --all-features

# Run linter checks
cargo clippy --all-features --package <package-name>
```

For comprehensive feature testing, consider using `cargo-hack` with `--feature-powerset` to test all feature combinations.

#### Async Testing

For async code:

```bash
# Run async tests with tokio
cargo test --package <package-name> -- --nocapture

# Run a specific async test
cargo test --package <package-name> test_name -- --nocapture
```

Remember to use `#[tokio::test]` for async test functions.

### Task Completion

- Mark completed tasks in TASK.md immediately after finishing them.
- Add new sub-tasks or TODOs discovered during development to TASK.md under a "Discovered During Work" section.
- Document any unexpected challenges or decisions made during implementation.

### Style & Conventions

- Follow the Rust API Guidelines and standard Rust code style.
- Use rustfmt and clippy to maintain consistent code style and catch common issues.
- Maintain idiomatic Rust practices (ownership, borrowing, error handling with Result).
- Write clear documentation comments for public APIs using rustdoc format.
- Use snake_case for variables and functions, PascalCase for types and structs.
- Use expressive variable names that convey intent (e.g., `is_ready`, `has_data`).
- **Do not create mod.rs files** - use the new module style.

#### Clean Code Principles

1. **Constants Over Magic Numbers**
   - Replace hard-coded values with named constants
   - Use descriptive constant names that explain the value's purpose
   - Keep constants at the top of the file or in a dedicated constants file

2. **Meaningful Names**
   - Variables, functions, and structures should reveal their purpose
   - Names should explain why something exists and how it's used
   - Avoid abbreviations unless they're universally understood

3. **Smart Comments**
   - Don't comment on what the code does - make the code self-documenting
   - Use comments to explain why something is done a certain way
   - Document APIs, complex algorithms, and non-obvious side effects

4. **Single Responsibility**
   - Each function should do exactly one thing
   - Functions should be small and focused
   - If a function needs a comment to explain what it does, it should be split

5. **DRY (Don't Repeat Yourself)**
   - Extract repeated code into reusable functions
   - Share common logic through proper abstraction
   - Maintain single sources of truth

6. **Clean Structure**
   - Keep related code together
   - Organize code in a logical hierarchy
   - Use consistent file and folder naming conventions

7. **Encapsulation**
   - Hide implementation details
   - Expose clear interfaces
   - Move nested conditionals into well-named functions

### Documentation & Explainability

- Update README.md when new features are added, dependencies change, or setup steps are modified.
- Comment non-obvious code and ensure everything is understandable to a mid-level Rust developer.
- When writing complex logic, add an inline `// Reason:` comment explaining the why, not just the what.
- Document error handling strategies and edge cases.

#### Rust Documentation Best Practices

1. **Function Documentation Structure**:
   - Begin with a clear, single-line summary of what the function does
   - Include a detailed description of the function's behavior
   - For simple functions (0-2 parameters), describe parameters inline in the main description
   - For complex functions (3+ parameters), use an explicit "# Arguments" section with bullet points
   - Always describe return values in the main description text, not in a separate section
   - Document error conditions with an explicit "# Errors" section

2. **Type Documentation**:
   - Begin with a clear, single-line summary of what the type represents
   - Explain the type's purpose, invariants, and usage patterns
   - Document struct fields with field-level doc comments
   - Document enum variants clearly

3. **Examples**:
   - Include practical examples for public APIs
   - Ensure examples compile and demonstrate typical usage patterns
   - For complex types/functions, show multiple usage scenarios

#### Creating Code Context in Rust

To get a better understanding of a Rust API:

```bash
# Generate documentation without opening it
cargo doc --no-deps --all-features --package <package-name>

# Generate documentation for the entire workspace
cargo doc --no-deps --all-features --workspace
```

These commands generate HTML documentation from the code and docstrings, providing a comprehensive view of the crate's structure, public API, and usage examples. This approach is particularly effective for:

1. Understanding a crate's organization and component relationships
2. Exploring available functions, types, and traits
3. Finding usage examples in doctest code blocks
4. Understanding error conditions and handling
5. Generating test data based on documented structures

### AI Behavior Rules

- Never assume missing context. Ask questions if uncertain about requirements or existing code.
- Never hallucinate libraries or functions – only use known, verified Rust crates as specified in DEPENDENCIES.md.
- Always confirm file paths and module names exist before referencing them in code or tests.
- Never delete or overwrite existing code unless explicitly instructed to or if part of a task from TASK.md.
- When updating existing code, preserve the original architecture and design patterns unless instructions specify otherwise.