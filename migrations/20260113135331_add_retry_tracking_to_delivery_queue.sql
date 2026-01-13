-- Add retry tracking columns to issue_delivery_queue
ALTER TABLE issue_delivery_queue
ADD COLUMN attempt_count INTEGER NOT NULL DEFAULT 0,
ADD COLUMN last_attempted_at TIMESTAMPTZ,
ADD COLUMN error_message TEXT;
