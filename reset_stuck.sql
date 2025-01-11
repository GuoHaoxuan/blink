UPDATE tasks
SET
    status = 'Pending',
    updated_at = DATETIME ('now'),
    retry_times = retry_times + 1
WHERE
    status = 'Running';
