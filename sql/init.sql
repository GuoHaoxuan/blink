CREATE TABLE
    IF NOT EXISTS task (
        satellite TEXT NOT NULL, -- 要处理的卫星
        detector TEXT NOT NULL, -- 要处理的探测器
        time TEXT NOT NULL, -- 要处理的时间
        created_at TEXT NOT NULL, -- 创建时间
        updated_at TEXT NOT NULL, -- 修改时间
        retry_times INTEGER NOT NULL, -- 重试次数
        worker TEXT, -- 处理者
        status TEXT NOT NULL, -- 状态 Pending, Running, Finished, Failed
        error TEXT, -- 错误信息
        UNIQUE (satellite, detector, time) ON CONFLICT IGNORE,
        INDEX (time)
    );

CREATE TABLE
    IF NOT EXISTS signal (
        id TEXT,
        satellite TEXT NOT NULL, -- 要处理的卫星
        detector TEXT NOT NULL, -- 要处理的探测器
        start_full TEXT NOT NULL, -- 开始时间
        start_best TEXT NOT NULL, -- 最佳开始时间
        stop_full TEXT NOT NULL, -- 结束时间
        stop_best TEXT NOT NULL, -- 最佳结束时间
        peak TEXT NOT NULL, -- 峰值
        duration_full REAL NOT NULL, -- 持续时间
        duration_best REAL NOT NULL, -- 最佳持续时间
        false_positive REAL NOT NULL, -- 年误触发个数
        false_positive_per_year REAL NOT NULL, -- 年误触发个数
        count_unfiltered_full INTEGER NOT NULL, -- 事件个数
        count_unfiltered_best INTEGER NOT NULL, -- 最佳事件个数
        count_filtered_full INTEGER NOT NULL, -- 有效事件个数
        count_filtered_best INTEGER NOT NULL, -- 最佳有效事件个数
        background REAL NOT NULL, -- 每秒本底
        flux_unfiltered_full REAL NOT NULL, -- 每秒通量
        flux_unfiltered_best REAL NOT NULL, -- 最佳每秒通量
        flux_filtered_full REAL NOT NULL, -- 有效事件每秒通量
        flux_filtered_best REAL NOT NULL, -- 最佳有效事件每秒通量
        events TEXT NOT NULL, -- 事件
        light_curve_1s_unfiltered TEXT NOT NULL, -- 光变曲线
        light_curve_1s_filtered TEXT NOT NULL, -- 有效事件光变曲线
        light_curve_100ms_unfiltered TEXT NOT NULL, -- 100ms 光变曲线
        light_curve_100ms_filtered TEXT NOT NULL, -- 有效事件 100ms 光变曲线
        longitude REAL NOT NULL, -- 经度
        latitude REAL NOT NULL, -- 纬度
        altitude REAL NOT NULL, -- 高度
        q1 REAL NOT NULL, -- Q1
        q2 REAL NOT NULL, -- Q2
        q3 REAL NOT NULL, -- Q3
        orbit TEXT NOT NULL, -- 轨道
        lightnings TEXT NOT NULL, -- 闪电
        associated_lightning_count INTEGER NOT NULL, -- 关联闪电个数
        coincidence_probability REAL NOT NULL, -- 偶然符合概率
        mean_solar_time TEXT NOT NULL, -- 平均太阳时
        apparent_solar_time TEXT NOT NULL, -- 视太阳时
        day_of_year INTEGER NOT NULL, -- 年中的第几天
        month INTEGER NOT NULL, -- 月份
        solar_zenith_angle REAL NOT NULL, -- 太阳天顶角
        solar_zenith_angle_at_noon REAL NOT NULL, -- 中午太阳天顶角
        solar_azimuth_angle REAL NOT NULL, -- 太阳方位角
        UNIQUE (satellite, detector, start_full) ON CONFLICT IGNORE,
        UNIQUE (id) ON CONFLICT IGNORE
    );

CREATE TABLE
    IF NOT EXISTS statistics (
        time TEXT NOT NULL, -- 要处理的时间
        what TEXT NOT NULL, -- 要处理的内容
        created_at TEXT NOT NULL, -- 创建时间
        updated_at TEXT NOT NULL, -- 修改时间
        retry_times INTEGER NOT NULL, -- 重试次数
        worker TEXT, -- 处理者
        status TEXT NOT NULL, -- 状态 Pending, Running, Finished, Failed
        error TEXT, -- 错误信息
        value TEXT, -- 统计值
        UNIQUE (time, what) ON CONFLICT IGNORE
    );
