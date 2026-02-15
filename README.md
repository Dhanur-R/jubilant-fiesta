# Linkr

A minimal, secure, and intelligent URL shortener built with Rust and Supabase.

## Features

- Lightning-fast Rust backend (Axum framework)
- Brutalist modern UI (Inter + JetBrains Mono fonts)
- Security hardened (rate limiting, SSRF protection, CSP headers)
- Smart deduplication (reuses existing short codes for same URLs)
- Auto-cleanup (removes links not accessed in 1 year)
- Supabase REST API (free forever, no database expiry)
- Production-ready (optimized for free tier)

## Quick Setup

### 1. Database Setup (Supabase)

Run this in your Supabase SQL Editor:

```sql
-- Create links table
CREATE TABLE IF NOT EXISTS links (
    id BIGSERIAL PRIMARY KEY,
    original_url TEXT NOT NULL,
    short_code TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_accessed TIMESTAMPTZ
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_links_short_code ON links(short_code);
CREATE INDEX IF NOT EXISTS idx_links_original_url ON links(original_url);
CREATE INDEX IF NOT EXISTS idx_links_last_accessed ON links(last_accessed);
CREATE INDEX IF NOT EXISTS idx_links_created_at ON links(created_at);
```

### 2. Environment Configuration

Update `.env` with your credentials:

```bash
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_KEY=your-service-role-key-here
PUBLIC_BASE_URL=https://linkr.dhanur.me
PORT=3000
RUST_LOG=linkr=info,tower_http=info
```

Get your `service_role` key from: Supabase Dashboard → Settings → API

### 3. Run Locally

```bash
cargo run
```

Visit: http://localhost:3000

### 4. Deploy to Render

1. Push to GitHub
2. Create new Web Service on Render
3. Set environment variables (same as `.env` above)
4. Deploy!

## API Endpoints

- `GET /` - Homepage with URL shortener
- `POST /shorten` - Create/retrieve short link (rate limited: 10/min)
- `GET /:code` - Redirect to original URL (updates last_accessed)
- `GET /health` - Health check endpoint

## Smart Features

### URL Deduplication
Shortening the same URL twice returns the same short code (saves database space).

### Intelligent Cleanup
Links not accessed in 1 year are automatically deleted daily. Runs at app startup + every 24 hours.

### Security
- Rate limiting: 10 requests/minute per IP
- SSRF protection: Blocks internal/private IPs
- Content Security Policy headers
- Input validation (max 2048 chars)
- HTTPS-only in production

## Tech Stack

- **Backend**: Rust + Axum + Tokio
- **Database**: Supabase (PostgreSQL via REST API)
- **Frontend**: HTMX + Tailwind CSS
- **Templates**: Askama
- **Fonts**: Inter (UI) + JetBrains Mono (code)

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Production build
cargo build --release

# Run production binary
./target/release/linkr
```

## Architecture

```
src/
├── config.rs       - Centralized constants
├── db.rs          - Supabase REST API client
├── http_helpers.rs - Template rendering utilities
├── main.rs        - Application entry point
├── middleware.rs  - Rate limiting & security headers
├── models.rs      - Data structures
├── routes.rs      - HTTP handlers
└── validation.rs  - URL & IP validation

templates/
├── base.html              - Base layout
├── index.html             - Homepage
└── partials/
    ├── form.html          - URL input form
    └── result.html        - Shortened link result
```

## Free Tier Optimization

This project is optimized for free tiers:
- **Supabase Free**: 500MB storage, shared CPU → Uses REST API (lightweight)
- **Render Free**: 750h/month → Cleanup runs in-app (uses Render CPU, not Supabase)

## License

MIT
