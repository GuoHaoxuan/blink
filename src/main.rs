mod search;

use core::str::FromStr;
use fitsio::FitsFile;
use hifitime::Epoch;
use itertools::Itertools;
use search::record;

fn main() {
    let filenames = [
        "current/glg_tte_n0_230101_00z_v00.fit.gz",
        "current/glg_tte_n1_230101_00z_v00.fit.gz",
        "current/glg_tte_n2_230101_00z_v00.fit.gz",
        "current/glg_tte_n3_230101_00z_v00.fit.gz",
        "current/glg_tte_n4_230101_00z_v00.fit.gz",
        "current/glg_tte_n5_230101_00z_v00.fit.gz",
        "current/glg_tte_n6_230101_00z_v00.fit.gz",
        "current/glg_tte_n7_230101_00z_v00.fit.gz",
        "current/glg_tte_n8_230101_00z_v00.fit.gz",
        "current/glg_tte_n9_230101_00z_v00.fit.gz",
        "current/glg_tte_na_230101_00z_v00.fit.gz",
        "current/glg_tte_nb_230101_00z_v00.fit.gz",
    ];
    let mut fptr = filenames
        .iter()
        .map(|filename| FitsFile::open(filename).unwrap())
        .collect::<Vec<_>>();
    let events = fptr
        .iter_mut()
        .map(|fptr| fptr.hdu("EVENTS").unwrap())
        .collect::<Vec<_>>();
    let time = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_col::<f64>(fptr, "TIME").unwrap())
        .collect::<Vec<_>>();
    let pha = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_col::<u8>(fptr, "PHA").unwrap())
        .collect::<Vec<_>>();
    let date_obs = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_key::<String>(fptr, "DATE-OBS").unwrap())
        .map(|date_obs| Epoch::from_str(&date_obs).unwrap())
        .collect::<Vec<_>>();
    let date_end = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_key::<String>(fptr, "DATE-END").unwrap())
        .map(|date_end| Epoch::from_str(&date_end).unwrap())
        .collect::<Vec<_>>();
    let t_start = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_key::<f64>(fptr, "TSTART").unwrap())
        .collect::<Vec<_>>();
    let t_stop = events
        .iter()
        .zip(fptr.iter_mut())
        .map(|(events, fptr)| events.read_key::<f64>(fptr, "TSTOP").unwrap())
        .collect::<Vec<_>>();

    let gti = fptr
        .iter_mut()
        .map(|fptr| fptr.hdu("GTI").unwrap())
        .collect::<Vec<_>>();
    let gti_start = gti
        .iter()
        .zip(fptr.iter_mut())
        .map(|(gti, fptr)| gti.read_col::<f64>(fptr, "START").unwrap())
        .collect::<Vec<_>>();
    let gti_stop = gti
        .iter()
        .zip(fptr.iter_mut())
        .map(|(gti, fptr)| gti.read_col::<f64>(fptr, "STOP").unwrap())
        .collect::<Vec<_>>();

    assert!(gti_start.iter().all(|x| x.len() == gti_start[0].len()));
    assert!(gti_stop.iter().all(|x| x.len() == gti_stop[0].len()));

    let gti_start = (0..gti_start[0].len())
        .map(|i| {
            gti_start
                .iter()
                .map(|x| x[i])
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
        })
        .collect::<Vec<_>>();
    let gti_stop = (0..gti_stop[0].len())
        .map(|i| {
            gti_stop
                .iter()
                .map(|x| x[i])
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
        })
        .collect::<Vec<_>>();

    assert!(gti_start.len() == gti_stop.len());

    assert!(date_obs.iter().all(|x| x == &date_obs[0]));
    let date_obs = date_obs[0];

    let time = time
        .into_iter()
        .flatten()
        .sorted_by(|a, b| a.partial_cmp(b).unwrap())
        .collect::<Vec<_>>();

    let results = (0..gti_start.len())
        .flat_map(|i| {
            search::algorithms::search_all(&time, gti_start[i], gti_stop[i], 100, 20.0, 8)
                .into_iter()
                .sorted_by(|a, b| a.start.partial_cmp(&b.start).unwrap())
                .coalesce(|prev, next| {
                    if prev.mergeable(&next, 0) {
                        Ok(prev.merge(&next))
                    } else {
                        Err((prev, next))
                    }
                })
        })
        .map(|trigger| record::Record::new(&trigger, date_obs))
        .collect::<Vec<_>>();
    for record in results {
        println!("{:?}", record);
    }
}
