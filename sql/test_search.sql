INSERT INTO
    task (
        satellite,
        detector,
        time,
        created_at,
        updated_at,
        retry_times,
        status
    )
VALUES
    (
        'HXMT',
        'HE',
        '2017-08-24 10:00:00',
        DATETIME ('now'),
        DATETIME ('now'),
        0,
        'Pending'
    )
