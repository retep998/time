
pub use self::inner::*;

#[cfg(unix)]
mod inner {
    use libc::{c_int, c_long, c_char, time_t};
    use std::mem;
    use ::Tm;

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub use self::mac::*;
    #[cfg(all(not(target_os = "macos"), not(target_os = "ios")))]
    pub use self::unix::*;

    /// ctime's `tm`
    #[repr(C)]
    struct tm {
        tm_sec: c_int,
        tm_min: c_int,
        tm_hour: c_int,
        tm_mday: c_int,
        tm_mon: c_int,
        tm_year: c_int,
        tm_wday: c_int,
        tm_yday: c_int,
        tm_isdst: c_int,
        tm_gmtoff: c_long,
        tm_zone: *const c_char,
    }

    fn rust_tm_to_tm(rust_tm: &Tm, tm: &mut tm) {
        tm.tm_sec = rust_tm.tm_sec;
        tm.tm_min = rust_tm.tm_min;
        tm.tm_hour = rust_tm.tm_hour;
        tm.tm_mday = rust_tm.tm_mday;
        tm.tm_mon = rust_tm.tm_mon;
        tm.tm_year = rust_tm.tm_year;
        tm.tm_wday = rust_tm.tm_wday;
        tm.tm_yday = rust_tm.tm_yday;
        tm.tm_isdst = rust_tm.tm_isdst;
    }

    fn tm_to_rust_tm(tm: &tm, utcoff: i32, rust_tm: &mut Tm) {
        rust_tm.tm_sec = tm.tm_sec;
        rust_tm.tm_min = tm.tm_min;
        rust_tm.tm_hour = tm.tm_hour;
        rust_tm.tm_mday = tm.tm_mday;
        rust_tm.tm_mon = tm.tm_mon;
        rust_tm.tm_year = tm.tm_year;
        rust_tm.tm_wday = tm.tm_wday;
        rust_tm.tm_yday = tm.tm_yday;
        rust_tm.tm_isdst = tm.tm_isdst;
        rust_tm.tm_utcoff = utcoff;
    }

    extern {
        fn gmtime_r(time_p: *const time_t, result: *mut tm) -> *mut tm;
        fn localtime_r(time_p: *const time_t, result: *mut tm) -> *mut tm;
        fn timegm(tm: *const tm) -> time_t;
        fn mktime(tm: *const tm) -> time_t;
    }

    pub fn time_to_utc_tm(sec: i64, tm: &mut Tm) {
        unsafe {
            let mut out = mem::zeroed();
            gmtime_r(&sec, &mut out);
            tm_to_rust_tm(&out, 0, tm);
        }
    }

    pub fn time_to_local_tm(sec: i64, tm: &mut Tm) {
        unsafe {
            let mut out = mem::zeroed();
            localtime_r(&sec, &mut out);
            tm_to_rust_tm(&out, out.tm_gmtoff as i32, tm);
        }
    }

    pub fn utc_tm_to_time(rust_tm: &Tm) -> i64 {
        let mut tm = unsafe { mem::zeroed() };
        rust_tm_to_tm(rust_tm, &mut tm);
        unsafe { timegm(&tm) }
    }

    pub fn local_tm_to_time(rust_tm: &Tm) -> i64 {
        let mut tm = unsafe { mem::zeroed() };
        rust_tm_to_tm(rust_tm, &mut tm);
        unsafe { mktime(&tm) }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    mod mac {
        use libc::{timeval, timezone, c_int, mach_timebase_info};
        use std::sync::{Once, ONCE_INIT};

        extern {
            fn gettimeofday(tp: *mut timeval, tzp: *mut timezone) -> c_int;
            fn mach_absolute_time() -> u64;
            fn mach_timebase_info(info: *mut mach_timebase_info) -> c_int;
        }

        pub fn info() -> &'static mach_timebase_info {
            static mut INFO: mach_timebase_info = mach_timebase_info {
                numer: 0,
                denom: 0,
            };
            static ONCE: Once = ONCE_INIT;

            unsafe {
                ONCE.call_once(|| {
                    mach_timebase_info(&mut INFO);
                });
                &INFO
            }
        }

        pub fn get_time() -> (i64, i32) {
            use std::ptr;
            let mut tv = libc::timeval { tv_sec: 0, tv_usec: 0 };
            unsafe { gettimeofday(&mut tv, ptr::null_mut()); }
            (tv.tv_sec as i64, tv.tv_usec * 1000)
        }

        pub fn get_precise_ns() -> u64 {
            unsafe {
                let time = mach_absolute_time();
                let info = info();
                time * info.numer as u64 / info.denom as u64
            }
        }
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "ios")))]
    mod unix {
        use libc::{self, c_int, timespec};

        #[cfg(all(not(target_os = "android"),
                  not(target_os = "bitrig"),
                  not(target_os = "nacl"),
                  not(target_os = "openbsd")))]
        #[link(name = "rt")]
        extern {}

        extern {
            pub fn clock_gettime(clk_id: c_int, tp: *mut timespec) -> c_int;
        }

        pub fn get_time() -> (i64, i32) {
            let mut tv = libc::timespec { tv_sec: 0, tv_nsec: 0 };
            unsafe { clock_gettime(libc::CLOCK_REALTIME, &mut tv); }
            (tv.tv_sec as i64, tv.tv_nsec as i32)
        }

        pub fn get_precise_ns() -> u64 {
            let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
            unsafe {
                clock_gettime(libc::CLOCK_MONOTONIC, &mut ts);
            }
            (ts.tv_sec as u64) * 1000000000 + (ts.tv_nsec as u64)
        }
    }
}

#[cfg(windows)]
#[allow(non_snake_case)]
mod inner {
    use libc::{self, WORD, DWORD, LONG, WCHAR};
    use ::{Tm};
    use std::mem;
    use std::sync::{Once, ONCE_INIT};

    pub fn frequency() -> libc::LARGE_INTEGER {
        static mut FREQUENCY: libc::LARGE_INTEGER = 0;
        static ONCE: Once = ONCE_INIT;

        unsafe {
            ONCE.call_once(|| {
                libc::QueryPerformanceFrequency(&mut FREQUENCY);
            });
            FREQUENCY
        }
    }

    #[repr(C)]
    struct SystemTime {
        wYear: WORD,
        wMonth: WORD,
        wDayOfWeek: WORD,
        wDay: WORD,
        wHour: WORD,
        wMinute: WORD,
        wSecond: WORD,
        wMilliseconds: WORD,
    }

    #[repr(C)]
    struct FileTime {
        dwLowDateTime: DWORD,
        dwHighDateTime: DWORD,
    }

    const HECTONANOSECS_IN_SEC: u64 = 10_000_000;
    const HECTONANOSEC_TO_UNIX_EPOCH: u64 = 11_644_473_600 * HECTONANOSECS_IN_SEC;

    fn time_to_file_time(sec: i64) -> FileTime {
        let t = (sec as u64 * HECTONANOSECS_IN_SEC) + HECTONANOSEC_TO_UNIX_EPOCH;
        FileTime {
            dwLowDateTime: t as DWORD,
            dwHighDateTime: (t >> 32) as DWORD
        }
    }

    fn file_time_to_nsec(ft: &FileTime) -> i32 {
        let t = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
        (((t - HECTONANOSEC_TO_UNIX_EPOCH) % HECTONANOSECS_IN_SEC) * 100) as i32
    }

    fn file_time_to_time(ft: &FileTime) -> i64 {
        let t = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
        ((t - HECTONANOSEC_TO_UNIX_EPOCH) / HECTONANOSECS_IN_SEC) as i64
    }

    fn tm_to_system_time(tm: &Tm) -> SystemTime {
        let mut sys = unsafe { mem::zeroed() };
        sys.wSecond = tm.tm_sec as WORD;
        sys.wMinute = tm.tm_min as WORD;
        sys.wHour = tm.tm_hour as WORD;
        sys.wDay = tm.tm_mday as WORD;
        sys.wDayOfWeek = tm.tm_wday as WORD;
        sys.wMonth = (tm.tm_mon + 1) as WORD;
        sys.wYear = (tm.tm_year + 1900) as WORD;
        sys
    }

    fn system_time_to_tm(sys: &SystemTime, tm: &mut Tm) {
        tm.tm_sec = sys.wSecond as i32;
        tm.tm_min = sys.wMinute as i32;
        tm.tm_hour = sys.wHour as i32;
        tm.tm_mday = sys.wDay as i32;
        tm.tm_wday = sys.wDayOfWeek as i32;
        tm.tm_mon = (sys.wMonth - 1) as i32;
        tm.tm_year = (sys.wYear - 1900) as i32;
        tm.tm_yday = yday(tm.tm_year, tm.tm_mon + 1, tm.tm_mday);

        fn yday(year: i32, month: i32, day: i32) -> i32 {
            let leap = if month > 2 {
                if year % 4 == 0 { 1 } else { 2 }
            } else {
                0
            };
            let july = if month > 7 { 1 } else { 0 };

            (month - 1) * 30 + month / 2 + (day - 1) - leap + july
        }
    }

    #[repr(C)]
    pub struct TimeZoneInfo {
        Bias: LONG,
        StandardName: [WCHAR; 32],
        StandardDate: SystemTime,
        StandardBias: LONG,
        DaylightName: [WCHAR; 32],
        DaylightDate: SystemTime,
        DaylightBias: LONG,
    }

    extern "system" {
        fn GetSystemTimeAsFileTime(out: *mut FileTime);
        fn FileTimeToSystemTime(ft: *const FileTime, out: *mut SystemTime) -> bool;
        fn SystemTimeToFileTime(sys: *const SystemTime, ft: *mut FileTime) -> bool;
        fn SystemTimeToTzSpecificLocalTime(tz: *const TimeZoneInfo, utc: *const SystemTime, local: *mut SystemTime) -> bool;
        fn TzSpecificLocalTimeToSystemTime(tz: *const TimeZoneInfo, local: *const SystemTime, utc: *mut SystemTime) -> bool;
    }
 
    pub fn time_to_utc_tm(sec: i64, tm: &mut Tm) {
        let mut out = unsafe { mem::zeroed() };
        let ft = time_to_file_time(sec);
        unsafe {
            FileTimeToSystemTime(&ft, &mut out);
        }
        system_time_to_tm(&out, tm);
        tm.tm_utcoff = 0;
    }

    pub fn time_to_local_tm(sec: i64, tm: &mut Tm) {
        let mut out = unsafe { mem::zeroed() };
        let tz = time_zone(None);
        let ft = time_to_file_time(sec);
        unsafe {
            FileTimeToSystemTime(&ft, &mut out);
            SystemTimeToTzSpecificLocalTime(tz, &out, &mut out);
            system_time_to_tm(&out, tm);
            tm.tm_utcoff = -(*tz).Bias * 60;
        }
    }

    pub fn utc_tm_to_time(tm: &Tm) -> i64 {
        unsafe {
            let mut ft = mem::zeroed();
            let sys_time = tm_to_system_time(tm);
            SystemTimeToFileTime(&sys_time, &mut ft);
            file_time_to_time(&ft)
        }
    }

    pub fn local_tm_to_time(tm: &Tm) -> i64 {
        unsafe {
            let mut ft = mem::zeroed();
            let mut utc = mem::zeroed();
            let sys_time = tm_to_system_time(tm);
            TzSpecificLocalTimeToSystemTime(time_zone(None), &sys_time, &mut utc);
            SystemTimeToFileTime(&utc, &mut ft);
            file_time_to_time(&ft)
        }
    }

    pub fn get_time() -> (i64, i32) {
        unsafe {
            let mut ft = mem::zeroed();
            GetSystemTimeAsFileTime(&mut ft);
            (file_time_to_time(&ft), file_time_to_nsec(&ft))
        }
    }

    pub fn get_precise_ns() -> u64 {
        let mut ticks = 0;
        unsafe {
            assert!(libc::QueryPerformanceCounter(&mut ticks) == 1);
        }
        mul_div_i64(ticks as i64, 1000000000, frequency() as i64) as u64

    }

    pub fn time_zone(bias: Option<i32>) -> *const TimeZoneInfo {
        use std::ptr;
        static mut TZ: Option<TimeZoneInfo> = None;
        static ONCE: Once = ONCE_INIT;

        unsafe {
            ONCE.call_once(|| {
                if let Some(bias) = bias {
                    let mut tz = TimeZoneInfo::default();
                    tz.Bias = bias;
                    TZ = Some(tz);
                }
            });

            match TZ {
                Some(ref tz) => tz,
                None => ptr::null()
            }
        }
    }

    // Computes (value*numer)/denom without overflow, as long as both
    // (numer*denom) and the overall result fit into i64 (which is the case
    // for our time conversions).
    pub fn mul_div_i64(value: i64, numer: i64, denom: i64) -> i64 {
        let q = value / denom;
        let r = value % denom;
        // Decompose value as (value/denom*denom + value%denom),
        // substitute into (value*numer)/denom and simplify.
        // r < denom, so (denom*numer) is the upper bound of (r*numer)
        q * numer + r * numer / denom
    }

    #[test]
    fn test_muldiv() {
        assert_eq!(mul_div_i64( 1_000_000_000_001, 1_000_000_000, 1_000_000),
                   1_000_000_000_001_000);
        assert_eq!(mul_div_i64(-1_000_000_000_001, 1_000_000_000, 1_000_000),
                   -1_000_000_000_001_000);
        assert_eq!(mul_div_i64(-1_000_000_000_001,-1_000_000_000, 1_000_000),
                   1_000_000_000_001_000);
        assert_eq!(mul_div_i64( 1_000_000_000_001, 1_000_000_000,-1_000_000),
                   -1_000_000_000_001_000);
        assert_eq!(mul_div_i64( 1_000_000_000_001,-1_000_000_000,-1_000_000),
                   1_000_000_000_001_000);
    }
}


