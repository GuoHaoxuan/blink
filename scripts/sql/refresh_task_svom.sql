INSERT
OR IGNORE INTO task (
    satellite,
    detector,
    time,
    created_at,
    updated_at,
    retry_times,
    status
)
WITH RECURSIVE
    hours AS (
        SELECT
            DATETIME ('2024-06-22 00:00:00') AS hour
        UNION ALL
        SELECT
            DATETIME (hour, '+1 hour')
        FROM
            hours
        WHERE
            hour < DATETIME ('now')
    )
SELECT
    'SVOM' AS satellite,
    'GRM' AS detector,
    hour AS time,
    DATETIME ('now') AS created_at,
    DATETIME ('now') AS updated_at,
    0 AS retry_times,
    'Pending' AS status
FROM
    hours;
