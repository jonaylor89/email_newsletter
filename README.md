# Email Newsletter

Email newsletter service built with Rust, Axum, and PostgreSQL.

## Setup

```bash
# Start PostgreSQL and Redis
./scripts/init_db.sh
./scripts/init_redis.sh

# Run migrations
sqlx migrate run

# Run the application
cargo run
```

## Testing

```bash
cargo test
```

## Architecture

- **API Server**: Subscription and newsletter endpoints
- **Email Delivery Worker**: Background queue with retry logic
- **Idempotency Cleanup Worker**: Daily cleanup of expired keys

## Templates

Email and web templates use Askama and are located in `templates/`.
