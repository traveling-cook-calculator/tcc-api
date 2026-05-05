# Traveling Cook Calculator API

A RESTful API for managing cooking teams, meal plans, courses, and culinary events built with **Rust**, **Axum**, **Diesel**, and **PostgreSQL**.

## Overview

The Traveling Cook Calculator API provides comprehensive backend services for organizing and managing traveling cooking events. It handles team management, meal planning, course organization, event coordination, and sharing capabilities with enterprise-grade security and observability.

## Features

- 🔐 **JWT Authentication** via Auth0 with permission-based access control
- 📋 **Team & Member Management** with flexible team configurations
- 🍽️ **Meal Planning** with course organization and JSONB-based configuration
- 📍 **Address & Location Management** for event coordination
- 📊 **Event Tracking** for cook-and-runs with comprehensive metadata
- 🔗 **Content Sharing** with granular permission controls
- ⚡ **Rate Limiting** (5 req/sec per IP) with burst allowance
- 📡 **OpenTelemetry Integration** for distributed tracing via Jaeger
- ✅ **Comprehensive Testing** with integration tests and auth token caching
- 🗄️ **PostgreSQL with Diesel ORM** for robust data persistence

## Quick Start

### Prerequisites

- **Rust** 1.70+ (install via [rustup](https://rustup.rs/))
- **PostgreSQL** 12+
- **Diesel CLI** (`cargo install diesel_cli --no-default-features --features postgres`)
- **Node.js** (optional, for OpenAPI documentation)

### Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd tcc-api
   ```

2. **Configure environment**
   ```bash
   cp .env.example .env
   ```
   
   Required environment variables:
   - `DATABASE_URL` - PostgreSQL connection string
   - `AUTH0_DOMAIN` - Auth0 tenant domain
   - `AUTH0_CLIENT_ID` - Auth0 application ID
   - `RUST_LOG` - Logging level (default: `info`)
   - `JAEGER_ENDPOINT` - OpenTelemetry endpoint (optional)

3. **Run migrations**
   ```bash
   diesel migration run
   ```

4. **Build and run**
   ```bash
   cargo build --release
   cargo run --release
   ```

The server will start on `http://localhost:3000` by default.

## Development

### Build
```bash
cargo build
```

### Run Tests
```bash
# Run all tests
cargo test

# Run specific test module
cargo test api::team

# Run with output
cargo test --test api -- --nocapture
```

### Database Migrations
```bash
# Create new migration
diesel migration generate migration_name

# Run all pending migrations
diesel migration run

# Undo last migration
diesel migration redo
```

### Code Formatting & Linting
```bash
# Format code
cargo fmt

# Run clippy linter
cargo clippy -- -D warnings
```

## Project Structure

```
src/
├── main.rs                 # Server setup, middleware configuration
├── error.rs               # Global error handling with AppError enum
├── *.rs                   # Domain layer modules (team, course, plan, etc.)
├── db/                    # Database layer with Diesel queries
│   ├── mod.rs            # Connection pool and migrations
│   ├── models.rs         # Diesel Queryable/Insertable structs
│   ├── schema.rs         # Diesel schema definitions
│   └── *.rs              # Entity-specific queries
├── rest/                  # REST API layer
│   ├── mod.rs            # Router setup and route merging
│   ├── auth.rs           # JWT verification, Claims, permissions
│   ├── models.rs         # Request/Response DTOs with validation
│   ├── validated_json.rs # Custom JSON extractor with validation
│   └── *.rs              # Entity-specific HTTP handlers
└── migrations/            # Diesel schema migrations

tests/
├── api/                   # Integration tests
│   ├── mod.rs            # Test utilities and helpers
│   └── {entity}/         # Tests per entity
```

## Architecture

This project follows a **clean layered architecture** with clear separation of concerns:

```
REST Layer (HTTP handlers, DTOs, validation)
         ↓
Domain Layer (Business logic, orchestration)
         ↓
Database Layer (Diesel ORM queries)
```

**Key Principles:**
- ✅ Unidirectional dependencies (REST → Domain → DB)
- ✅ No reverse dependencies
- ✅ Three-level validation (HTTP extraction, DTO validation, business logic)
- ✅ Immediate error mapping at DB layer

For detailed architecture documentation, see [ARCHITECTURE.md](ARCHITECTURE.md).

For AI agent development guidance, see [AGENTS.md](AGENTS.md).

## CI/CD Pipeline

This project includes a comprehensive GitHub Actions workflow that automates testing, building, and releasing.

### Automated Workflows

**On every push:**
- ✅ Lint & format checks
- ✅ Unit & integration tests
- ✅ Multi-platform binary builds (Linux, macOS, Windows)
- ✅ Docker image build & push

**Main branch:**
- ✅ Creates production releases
- ✅ Tags Docker images as `latest`
- ✅ Uploads compiled binaries to GitHub Releases

**Feature branches:**
- ✅ Creates pre-releases
- ✅ Tags Docker images with branch name
- ✅ Builds all artifacts for testing

### Supported Platforms

**Binaries:**
- Linux (x86_64, ARM64)
- macOS (x86_64, ARM64)
- Windows (x86_64)

**Container:**
- `ghcr.io/{owner}/{repo}:latest` (main branch)
- `ghcr.io/{owner}/{repo}:v{version}` (all releases)

### Setup Requirements

Add these secrets to your GitHub repository settings:
- `AUTH0_DOMAIN` - Your Auth0 tenant domain
- `AUTH0_AUDIENCE` - Your Auth0 API audience

For detailed CI/CD documentation, see [.github/workflows/README.md](.github/workflows/README.md).

### Local Testing

```bash
# Start PostgreSQL for integration tests
docker-compose -f docker-compose.dev.yml up postgres

# Run all tests
cargo test

# Build Docker image
docker build -t tcc-api:dev .
```

## API Documentation

### OpenAPI/Swagger
The API is documented using OpenAPI 3.0 specification:

```bash
# View the spec
cat openapi/swagger.yml

# Start development server and visit
# http://localhost:3000/docs
```

### Base URL
- Development: `http://localhost:3000`
- Production: TBD

### Authentication
All endpoints require a valid JWT token from Auth0 (except public endpoints). Include the token in the `Authorization` header:

```bash
curl -H "Authorization: Bearer {token}" \
     http://localhost:3000/api/teams
```

### Key Endpoints

**Teams**
- `GET /api/teams` - List teams
- `POST /api/teams` - Create team
- `GET /api/teams/{id}` - Get team details
- `PUT /api/teams/{id}` - Update team
- `DELETE /api/teams/{id}` - Delete team

**Courses**
- `GET /api/courses` - List courses
- `POST /api/courses` - Create course
- `GET /api/courses/{id}` - Get course
- `PUT /api/courses/{id}` - Update course
- `DELETE /api/courses/{id}` - Delete course

**Plans**
- `GET /api/plans` - List plans
- `POST /api/plans` - Create plan
- `GET /api/plans/{id}` - Get plan
- `PUT /api/plans/{id}` - Update plan
- `DELETE /api/plans/{id}` - Delete plan

See OpenAPI spec for complete endpoint documentation.

## Error Handling

The API uses structured error responses with descriptive messages and HTTP status codes:

```json
{
  "error": "Team not found",
  "status": 404
}
```

**Common Status Codes:**
- `200` - OK
- `201` - Created
- `400` - Bad Request (validation error)
- `401` - Unauthorized (invalid/missing token)
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `409` - Conflict (business logic violation)
- `500` - Internal Server Error

## Database Schema

### Key Tables
- `teams` - Team entities with metadata
- `team_members` - Team membership records
- `courses` - Course definitions
- `plans` - Meal plans with JSONB configuration
- `addresses` - Address information (addresses/locations)
- `notes` - Shared notes for plans
- `points` - Points/locations within plans
- `sharing` - Sharing permissions and access control
- `cook_and_runs` - Event/execution records

### Special Features
- **Soft Deletes**: `deleted_at` column for recoverable deletions
- **JSONB Config**: Plans use JSONB for flexible configuration storage
- **Audit Timestamps**: `updated_at` tracked automatically
- **UUIDs**: All primary keys use UUID v4

## Middleware Stack

The API includes a comprehensive middleware stack (in order):

1. **CORS** - Cross-Origin Resource Sharing configuration
2. **Security Headers** - CSP, X-Frame-Options, X-Content-Type-Options
3. **Rate Limiting** - 5 requests/sec per IP (20 burst)
4. **Request Timeout** - 30 second timeout per request
5. **Body Size Limit** - Maximum 1 MiB payload
6. **Tracing** - Request/response logging to OpenTelemetry

## Observability

### Logging
Application logs are structured using the `tracing` crate:

```bash
# Set log level
RUST_LOG=debug cargo run

# Levels: trace, debug, info, warn, error
```

### Distributed Tracing
Traces are exported to Jaeger via OpenTelemetry:

```bash
# Configure Jaeger endpoint
JAEGER_ENDPOINT=http://localhost:14250 cargo run
```

Access traces at `http://localhost:16686` (if Jaeger is running).

## Performance Considerations

- **Connection Pooling**: R2D2 connection pool for PostgreSQL
- **Query Caching**: Auth0 JWKS keys cached on startup
- **Rate Limiting**: Per-IP rate limiting to prevent abuse
- **Soft Deletes**: Indexed `deleted_at` column for efficient filtering

## Testing

The project includes comprehensive integration tests:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific module
cargo test api::team

# Run single test
cargo test api::team::test_create_team
```

**Test Organization:**
- Tests are located in `tests/api/`
- Each entity has dedicated test module
- Auth tokens are cached via `OnceCell` for performance
- Test helpers provide team/course/plan creation utilities

## Deployment

### TBD
- Container configuration (Docker)
- Database migration strategy
- Environment-specific configurations
- CI/CD pipeline
- Monitoring and alerting

## Contributing

### Code Standards
- Follow Rust conventions and idioms
- Format code with `cargo fmt`
- Pass `cargo clippy` linter checks
- Write tests for new features
- Update documentation as needed

### Commit Messages
- Use clear, descriptive commit messages
- Reference issue numbers when applicable
- Follow conventional commit format where possible

### Pull Request Process
1. Fork the repository
2. Create a feature branch (`git checkout -b feature/description`)
3. Commit your changes (`git commit -am 'Add feature'`)
4. Push to the branch (`git push origin feature/description`)
5. Submit a Pull Request

## Troubleshooting

### Database Connection Issues
```bash
# Verify DATABASE_URL is set correctly
echo $DATABASE_URL

# Test connection
psql $DATABASE_URL -c "SELECT 1;"
```

### Migration Problems
```bash
# Check migration status
diesel migration list

# Revert and rerun migrations
diesel migration redo
```

### Auth0 Configuration
- Verify `AUTH0_DOMAIN` and `AUTH0_CLIENT_ID` are set
- Check that tenant is configured correctly
- Verify JWKS endpoint is accessible

### Rate Limiting Issues
- Default: 5 requests/sec per IP with 20 request burst
- Adjust in `src/main.rs` middleware configuration if needed

## License

This project is licensed under the [LICENSE](LICENSE) file. See file for details.

## Contact & Support

TBD - Add contact information, support channels, or issue tracking

## Resources

- [Full Architecture Documentation](ARCHITECTURE.md)
- [AI Agent Development Guide](AGENTS.md)
- [Diesel ORM Documentation](https://diesel.rs)
- [Axum Web Framework](https://docs.rs/axum)
- [Auth0 Documentation](https://auth0.com/docs)
- [OpenTelemetry Docs](https://opentelemetry.io)
