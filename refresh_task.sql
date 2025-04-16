INSERT
OR IGNORE INTO tasks (
    created_at,
    updated_at,
    retry_times,
    worker,
    status,
    error,
    time,
    satellite,
    detector
)
WITH RECURSIVE
    hours AS (
        SELECT
            DATETIME ('2017-06-22 00:00:00') AS hour
        UNION ALL
        SELECT
            DATETIME (hour, '+1 hour')
        FROM
            hours
        WHERE
            hour < DATETIME ('now')
    )
SELECT
    DATETIME ('now') AS created_at,
    DATETIME ('now') AS updated_at,
    0 AS retry_times,
    '' AS worker,
    'Pending' AS status,
    '' AS error,
    hour AS time,
    'HXMT' AS satellite,
    'HE' AS detector
FROM
    hours;
