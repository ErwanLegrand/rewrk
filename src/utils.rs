const GIGABYTE: f64 = (1024 * 1024 * 1024) as f64;
const MEGABYTE: f64 = (1024 * 1024) as f64;
const KILOBYTE: f64 = 1024_f64;

/// Dirt simple div mod function.
pub fn div_mod(main: u64, divider: u64) -> (u64, u64) {
    let whole = main / divider;
    let rem = main % divider;

    (whole, rem)
}

pub fn format_data(data_size: f64) -> String {
    if data_size > GIGABYTE {
        format!("{:.2} GB", data_size / GIGABYTE)
    } else if data_size > MEGABYTE {
        format!("{:.2} MB", data_size / MEGABYTE)
    } else if data_size > KILOBYTE {
        format!("{:.2} KB", data_size / KILOBYTE)
    } else {
        format!("{:.2} B", data_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_mod() {
        let (whole, rem) = div_mod(10, 3);
        assert_eq!(whole, 3, "10 / 3 should be 3");
        assert_eq!(rem, 1, "10 % 3 should be 1");

        let (whole, rem) = div_mod(9, 3);
        assert_eq!(whole, 3, "9 / 3 should be 3");
        assert_eq!(rem, 0, "9 % 3 should be 0");
    }

    #[test]
    fn test_format_data_bytes() {
        let result = format_data(512.0);
        assert!(
            result.contains('B') && !result.contains("KB"),
            "512 bytes should display as B, got: {}",
            result
        );
    }

    #[test]
    fn test_format_data_kilobytes() {
        let result = format_data(2048.0); // 2 KB
        assert!(
            result.contains("KB"),
            "2048 bytes should display as KB, got: {}",
            result
        );
    }

    #[test]
    fn test_format_data_megabytes() {
        let result = format_data(2.0 * 1024.0 * 1024.0); // 2 MB
        assert!(
            result.contains("MB"),
            "2 MB should display as MB, got: {}",
            result
        );
    }

    #[test]
    fn test_format_data_gigabytes() {
        let result = format_data(2.0 * 1024.0 * 1024.0 * 1024.0); // 2 GB
        assert!(
            result.contains("GB"),
            "2 GB should display as GB, got: {}",
            result
        );
    }
}
