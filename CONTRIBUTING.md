# Contributing

Contributions welcome! This is an email newsletter service built with Rust, Actix-web, and PostgreSQL.

## Development Setup

```bash
# Clone the repository
git clone <repo-url>
cd email_newsletter

# Start PostgreSQL and Redis
./scripts/init_db.sh
./scripts/init_redis.sh

# Build the project
cargo build

# Run migrations
sqlx migrate run --database-url=postgres://postgres:password@localhost:5432/newsletter

# Run the application
cargo run
```

## Database Changes

If you modify database queries:

```bash
# Generate SQLx metadata for offline compilation
cargo sqlx prepare --database-url=postgres://postgres:password@localhost:5432/newsletter

# Create a new migration
sqlx migrate add <migration_name>
```

## Testing

```bash
# Run all tests
cargo test

# Run with logging output
TEST_LOG=1 cargo test

# Run a specific test
cargo test test_subscribe

# Run integration tests
cargo test --test health_check
```

## Checks

Before submitting a PR:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check compilation
cargo check --all-targets

# Build release to catch any issues
cargo build --release
```

## Project Structure

```
src/
├── routes/              # HTTP endpoints
│   ├── subscriptions.rs       # Subscription management
│   ├── subscriptions_confirm.rs # Email confirmation
│   └── admin/                 # Admin routes (newsletters, dashboard)
├── domain/              # Domain types with validation
│   ├── subscriber_email.rs
│   ├── subscriber_name.rs
│   ├── subscription_token.rs
│   └── password.rs
├── authentication/      # Auth middleware and password hashing
├── idempotency/        # Idempotency key handling
├── email_client.rs     # Postmark email integration
├── email_templates.rs  # Askama templates
├── issue_delivery_queue.rs # Background email worker
├── idempotency_cleanup.rs  # Background cleanup worker
├── configuration.rs    # Settings management
└── startup.rs          # Application initialization
```

## Architecture

The application runs three concurrent workers:
- **API Server**: HTTP endpoints for subscriptions and newsletter publishing
- **Email Delivery Worker**: Processes newsletter delivery queue with retry logic
- **Idempotency Cleanup Worker**: Daily cleanup of expired idempotency keys

## Email Templates

Email templates use Askama and are located in `templates/emails/`:
- `confirmation.html/txt` - New subscriber confirmation
- `already_subscribed.html/txt` - Duplicate subscription notification

After modifying templates, rebuild to recompile them.

## Database Schema

Key tables:
- `subscriptions` - Subscriber emails and confirmation status
- `subscription_tokens` - Email confirmation tokens
- `users` - Admin users with Argon2 hashed passwords
- `newsletter_issues` - Newsletter content
- `issue_delivery_queue` - Delivery tasks with retry tracking
- `dead_letter_queue` - Permanently failed deliveries
- `idempotency` - Request deduplication (30-day retention)

## Commit Messages

Use conventional commits with descriptive details:

```
feat: add parallel email sending with tokio JoinSet
fix: handle duplicate subscription attempts gracefully
docs: update installation instructions
test: add integration tests for confirmation flow
refactor: extract email template rendering
```

For multi-file changes, each commit should be self-contained and focused on one logical change.

## Version Control

This project uses `jj` (Jujutsu) for version control. Common commands:

```bash
# See changes
jj status

# Create a commit
jj commit -m "description"

# View commit history
jj log

# Create a pull request (requires gh CLI)
gh pr create
```

## AI Assistance

If using AI assistance (Claude, GitHub Copilot, etc.), please:
- Disclose it in your PR description
- Review generated code carefully for correctness and security
- Ensure all tests pass and code follows project patterns
- Verify database queries are safe from SQL injection
