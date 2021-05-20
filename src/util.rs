
struct ByteSize(u64);

impl std::fmt::Display for ByteSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SUFFIXES : [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

        let mut size = self.0 as f32;
        let mut i = 0;
        for _ in SUFFIXES.iter() {
            if size > 1000.0 {
                size /= 1000.0;
                i += 1;
            }
        }

        write!(f, "{:.1}{}", size, SUFFIXES[i])
    }
}