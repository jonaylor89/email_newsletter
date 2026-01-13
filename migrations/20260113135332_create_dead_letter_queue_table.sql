-- Create dead letter queue for failed deliveries
CREATE TABLE dead_letter_queue(
    newsletter_issue_id UUID NOT NULL,
    subscriber_email TEXT NOT NULL,
    attempt_count INTEGER NOT NULL,
    last_error TEXT NOT NULL,
    failed_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (newsletter_issue_id, subscriber_email)
);
