use rusqlite::{Connection, params};
use serde::Serialize;

#[derive(Serialize)]
struct Tgf {
    id: String,
    detector: String,
    start: String,
    duration: f64,
    confidence: f64,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    apparent_solar_time: String,
}

#[derive(Serialize)]
struct LastUpdate {
    #[serde(rename = "HXMT/HE")]
    hxmt: String,
}

#[derive(Serialize)]
struct Output {
    last_update: LastUpdate,
    tgfs: Vec<Tgf>,
}

fn main() {
    let conn = Connection::open("blink.db").unwrap();
    let times_to_be_number = conn
        .prepare(
            "
            SELECT rowid, start_best
            FROM signal
            WHERE id is NULL
            ORDER BY start_best
        ",
        )
        .unwrap()
        .query_map([], |row| {
            let rowid: i64 = row.get(0)?;
            let start_best: String = row.get(1)?;
            Ok((rowid, start_best))
        })
        .unwrap()
        .collect::<Result<Vec<(i64, String)>, _>>()
        .unwrap();
    times_to_be_number.iter().for_each(|(rowid, start_best)| {
        let new_id = match conn
            .prepare(
                "
                SELECT id
                FROM signal
                WHERE SUBSTR(start_best, 1, 10) = SUBSTR(?1, 1, 10)
                  AND id IS NOT NULL
                ORDER BY id DESC
                LIMIT 1
            ",
            )
            .unwrap()
            .query_row(params![start_best], |row| row.get::<_, String>(0))
            .ok()
        {
            Some(id) => {
                // id 是 070807AA 这种格式，字母范围是A-Z
                // start_best 是 2017-06-27T14:08:40.036956012Z 这种格式
                // 我想要在 id 中后面AA的地方增加1作为新的id
                if id.len() >= 2 {
                    let (prefix, suffix) = id.split_at(id.len() - 2);
                    let mut chars: Vec<char> = suffix.chars().collect();
                    if chars[1] == 'Z' {
                        if chars[0] == 'Z' {
                            // 如果是ZZ，panic
                            panic!("id overflow: {}", id);
                        } else {
                            // 如果是AZ，变成BA
                            chars[0] = ((chars[0] as u8) + 1) as char;
                            chars[1] = 'A';
                            format!("{}{}", prefix, chars.iter().collect::<String>())
                        }
                    } else {
                        // 否则直接后面加1
                        chars[1] = ((chars[1] as u8) + 1) as char;
                        format!("{}{}", prefix, chars.iter().collect::<String>())
                    }
                } else {
                    // 如果id长度小于2，panic
                    panic!("id too short: {}", id);
                }
            }
            None => {
                // 如果没有找到对应的id，就创建新的id，末尾是AA
                // start_best 是 2017-06-27T14:08:40.036956012Z 这种格式
                // 就变成 170627AA 这种格式
                let date_part = &start_best[2..10]; // 取出 17-06-27
                // 变成
                date_part.replace("-", "") + "AA"
            }
        };
        conn.execute(
            "UPDATE signal SET id = ?1 WHERE rowid = ?2",
            params![new_id, rowid],
        )
        .unwrap();
    });
    let tgfs = conn
        .prepare(
            "
            SELECT id,
                start_best,
                duration_best,
                false_positive_per_year,
                longitude,
                latitude,
                altitude,
                apparent_solar_time,
                satellite,
                detector
            FROM signal
            ORDER BY id
            ",
        )
        .unwrap()
        .query_map([], |row| {
            Ok(Tgf {
                id: row.get(0)?,
                detector: format!("{}/{}", row.get::<_, String>(8)?, row.get::<_, String>(9)?),
                start: row.get(1)?,
                duration: row.get(2)?,
                confidence: row.get(3)?,
                longitude: row.get(4)?,
                latitude: row.get(5)?,
                altitude: row.get(6)?,
                apparent_solar_time: row.get(7)?,
            })
        })
        .unwrap()
        .collect::<Result<Vec<Tgf>, _>>()
        .unwrap();
    let hxmt_last_update: String = conn
        .prepare(
            "
            SELECT MAX(time)
            FROM task
            WHERE satellite = 'HXMT'
            AND detector = 'HE'
            AND status = 'Finished'
            ",
        )
        .unwrap()
        .query_row([], |row| row.get(0))
        .unwrap();
    let output = Output {
        last_update: LastUpdate {
            hxmt: hxmt_last_update,
        },
        tgfs,
    };
    let json = serde_json::to_string_pretty(&output).unwrap();
    std::fs::write("tgfs.json", json).unwrap();
}
