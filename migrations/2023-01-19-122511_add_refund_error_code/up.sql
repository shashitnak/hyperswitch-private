ALTER TABLE refund
ADD IF NOT EXISTS refund_error_code TEXT DEFAULT NULL;
