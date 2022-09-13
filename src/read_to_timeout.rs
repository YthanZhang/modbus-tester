/// The [std::io::Read](std::io::Read) trait implements many input operation,
/// but is doesn't contain a simple read until timeout method
///
/// This trait provides [read_to_timeout](ReadToTimeout::read_to_timeout) and
/// [read_to_pattern_or_timeout](ReadToTimeout::read_to_pattern_or_timeout)
/// that have a default implementation for all types that implements
/// [std::io::Read](std::io::Read)
pub trait ReadToTimeout {
    /// Similar to [`read_to_end`](std::io::Read::read_to_end)
    ///
    /// But when timeout, instead of returning error, this function returns Ok(bytes_read)
    fn read_to_timeout(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize>;

    /// Similar to [`read_to_timeout`](ReadToTimeout::read_to_timeout)
    ///
    /// But when a specified pattern is reached, return Ok(bytes_read) immediately
    ///
    /// # Note
    /// If the provided buffer is non-empty, while **at least one byte** must be
    /// read before any pattern match, it is possible for pattern to match on
    /// old bytes.
    fn read_to_pattern_or_timeout(
        &mut self,
        buf: &mut Vec<u8>,
        pattern: &[u8],
    ) -> std::io::Result<usize>;
}

// impl ReadTimeout for all T that impl Read for T
impl<T: std::io::Read> ReadToTimeout for T {
    fn read_to_timeout(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let old_len = buf.len();

        match self.read_to_end(buf) {
            Ok(bytes_read) => Ok(bytes_read),
            Err(err) => match err.kind() {
                std::io::ErrorKind::TimedOut => Ok(buf.len() - old_len),
                _ => Err(err),
            },
        }
    }

    fn read_to_pattern_or_timeout(
        &mut self,
        buf: &mut Vec<u8>,
        pattern: &[u8],
    ) -> std::io::Result<usize> {
        let old_len = buf.len();

        loop {
            let mut byte = [0];
            match self.read(&mut byte) {
                Ok(_) => {
                    buf.push(byte[0]);
                    if buf.len() >= pattern.len()
                        && &buf[(buf.len() - pattern.len())..] == pattern
                    {
                        break Ok(buf.len() - old_len);
                    }
                }
                Err(err) => match err.kind() {
                    std::io::ErrorKind::TimedOut => {
                        break Ok(buf.len() - old_len);
                    }
                    _ => {
                        break Err(err);
                    }
                },
            }
        }
    }
}
