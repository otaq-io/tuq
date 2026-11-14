//! Today's local date as `YYYY-MM-DD` (what `date +%Y-%m-%d` printed), from the C library or Windows itself.

pub fn today() -> String {
    let (y, m, d) = imp::local_date();
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(unix)]
mod imp {
    use std::os::raw::{c_int, c_long};

    // The fields POSIX names, in their order; the platform's extra fields come after and get room to spare.
    #[repr(C)]
    struct Tm {
        tm_sec: c_int,
        tm_min: c_int,
        tm_hour: c_int,
        tm_mday: c_int,
        tm_mon: c_int,
        tm_year: c_int,
        tm_wday: c_int,
        tm_yday: c_int,
        tm_isdst: c_int,
        _rest: [u64; 8],
    }

    extern "C" {
        fn tzset();
        fn localtime_r(t: *const c_long, tm: *mut Tm) -> *mut Tm;
    }

    pub fn local_date() -> (i32, u32, u32) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as c_long)
            .unwrap_or(0);
        let mut tm = Tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 1,
            tm_mon: 0,
            tm_year: 70,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
            _rest: [0; 8],
        };
        // SAFETY: `secs` and `tm` are valid for the call; `Tm` has at least the room the C library writes.
        let ok = unsafe {
            tzset();
            !localtime_r(&secs, &mut tm).is_null()
        };
        if !ok {
            #[allow(clippy::useless_conversion)] // c_long is i32 on some targets
            return super::utc_date(i64::from(secs));
        }
        let _ = (
            tm.tm_sec,
            tm.tm_min,
            tm.tm_hour,
            tm.tm_wday,
            tm.tm_yday,
            tm.tm_isdst,
        );
        (tm.tm_year + 1900, (tm.tm_mon + 1) as u32, tm.tm_mday as u32)
    }
}

#[cfg(windows)]
mod imp {
    #[repr(C)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetLocalTime(st: *mut SystemTime);
    }

    pub fn local_date() -> (i32, u32, u32) {
        let mut st = SystemTime {
            year: 1970,
            month: 1,
            day_of_week: 0,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            milliseconds: 0,
        };
        // SAFETY: `st` is a valid SYSTEMTIME for the call to fill.
        unsafe { GetLocalTime(&mut st) };
        (i32::from(st.year), u32::from(st.month), u32::from(st.day))
    }
}

#[cfg(not(any(unix, windows)))]
mod imp {
    pub fn local_date() -> (i32, u32, u32) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        super::utc_date(i64::from(secs))
    }
}

/// The UTC date of a Unix time (days-to-civil, Howard Hinnant's algorithm).
#[allow(dead_code)]
fn utc_date(secs: i64) -> (i32, u32, u32) {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (yoe + era * 400 + i64::from(m <= 2)) as i32;
    (y, m, d)
}

#[cfg(test)]
mod tests {
    #[test]
    fn utc_date_known_days() {
        assert_eq!(super::utc_date(0), (1970, 1, 1));
        assert_eq!(super::utc_date(1_794_528_000), (2026, 11, 13));
    }

    #[test]
    fn today_has_the_shape() {
        let t = super::today();
        assert_eq!(t.len(), 10, "{t}");
        assert_eq!(&t[4..5], "-");
    }
}
