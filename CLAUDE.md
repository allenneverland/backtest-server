# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
- **Test Server**: Python Flask app for testing messages

### Development Patterns

1. **Builder Pattern**: Used extensively for complex object construction (e.g., ServerBuilder)
2. **Repository Pattern**: Data access abstraction in storage module
3. **Strategy Pattern**: For different validator and cleaner implementations
4. **Observer Pattern**: Event bus for loose coupling between components
5. **Sandbox Pattern**: Isolated execution environment for user strategies