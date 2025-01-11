UPDATE tasks
SET
    status = 'Running',
    updated_at = DATETIME ('now'),
    retry_times = retry_times + 1
WHERE
    updated_at < DATETIME ('now', '-1 day');
