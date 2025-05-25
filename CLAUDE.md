Commend use Traditional Chinese with correct UTF-8 format

Always use sequentialthinking mcp

## Gitlab

Only use the following labels:
### type label:
bug - Bug 修復
feature - 新功能
enhancement - 改進現有功能
refactor - 重構程式碼
documentation - 文件相關
test - 測試相關
security - 安全性問題

### priority label:
critical - 緊急修復
high - 高優先級
medium - 中等優先級
low - 低優先級

Only use the following milestone:
### milestone:
第一階段：專案設置和基礎架構 id: 6021454
第二階段：核心數據功能 id:6021455
第三階段：回測與執行模組 id:6021457
第四階段：策略與隔離運行時 id:6021459
第五階段：消息系統集成與伺服器功能 id:6021460
第六階段：集成與測試 id:6021461

## Common Development Commands

### Building and Running

```bash
# Build development version
cargo make docker-c cargo build

# Build release version  
cargo make docker-c cargo build-release

# Run the server (development)
cargo make docker-c cargo run

# Run the server (production)
cargo make docker-c cargo run-release

# Watch for changes and auto-build/test
cargo make docker-c cargo watch
```

### Testing and Quality

```bash
# Run all tests
cargo make docker-c cargo test-all

# Run unit tests
cargo make docker-c cargo test

# Run integration tests
cargo make test-integration

# Format code
cargo make docker-c cargo format

# Check code formatting
cargo make docker-c cargo format-check

# Run linter
cargo make docker-c cargo lint

# Generate coverage report
cargo make docker-c cargo coverage
```

### Database Management

```bash
# Run migrations using Rust binary
cargo make docker-c cargo run --bin migrate run

# Check migration status
cargo make docker-c cargo run --bin migrate status
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
cargo make docker-c cargo make run-example <example_name>

# Common examples:
cargo make docker-c cargo make run-example domain_types_demo
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
- Always run `cargo make docker-c cargo check` after editing code to catch compilation errors early.
- After updating any logic, check whether existing unit tests need to be updated. If so, update them.
- Write tests before fixing bugs to verify the issue and prevent regressions.
- Keep tests readable and maintainable.
- Test edge cases and error conditions thoroughly.

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

2. **Single Responsibility**
   - Each function should do exactly one thing
   - Functions should be small and focused
   - If a function needs a comment to explain what it does, it should be split

3. **DRY (Don't Repeat Yourself)**
   - Extract repeated code into reusable functions
   - Share common logic through proper abstraction
   - Maintain single sources of truth

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

### Behavior Rules

- Never assume missing context. Ask questions if uncertain about requirements or existing code.
- Never hallucinate libraries or functions – only use known, verified Rust crates as specified in DEPENDENCIES.md.
- Always confirm file paths and module names exist before referencing them in code or tests.
- When updating existing code, preserve the original architecture and design patterns unless instructions specify otherwise.