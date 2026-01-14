# TODO - Feature Roadmap

Self-hosted newsletter features, prioritized for maximum value with minimal dependencies.

## 🔴 High Priority

- [ ] **Analytics (opens/clicks)**
  - Tracking pixel for open detection
  - Redirect links for click tracking
  - Store metrics in PostgreSQL / Clickhouse
  - Dashboard to view per-issue stats

- [ ] **Unsubscribe handling**
  - One-click unsubscribe header (RFC 8058)
  - Unsubscribe landing page
  - List-Unsubscribe header in all emails
  - Required for CAN-SPAM/GDPR compliance

- [ ] **Subscriber import/export**
  - CSV import endpoint with validation
  - CSV export for full data portability
  - Preserve confirmation status on import

## 🟡 Medium Priority

- [ ] **Segmentation/tags**
  - Tags table linked to subscriptions
  - Filter subscribers by tag on send
  - Auto-tagging based on signup source

- [ ] **A/B testing**
  - Split audience for subject line tests
  - Track open rates per variant
  - Auto-select winner after threshold

- [ ] **Scheduling**
  - Add `scheduled_at` column to newsletter_issues
  - Background worker checks for scheduled sends
  - Admin UI for picking send time

- [ ] **Web archive**
  - Public `/archive` listing past issues
  - Individual issue pages at `/archive/:id`
  - SEO-friendly with meta tags

- [ ] **API keys**
  - API token generation in admin
  - Token auth middleware for programmatic access
  - Endpoints for subscribe/unsubscribe/send

## 🟢 Nice to Have

- [ ] **Polls/surveys**
  - Simple voting endpoint (no external deps)
  - Store results in PostgreSQL
  - Display results in follow-up emails

- [ ] **RSS-to-email**
  - Background worker polls configured RSS feeds
  - Auto-generate newsletter from new items
  - Configurable frequency

- [ ] **MJML support**
  - Compile MJML templates to HTML at build time
  - Responsive emails without manual table layouts

## 🧹 Housekeeping

- [ ] **Rate limiting**
  - Protect subscribe endpoint from abuse
  - In-memory or PostgreSQL-based limiter
