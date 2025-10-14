use std::collections::VecDeque;

/// Split into its own struct to be tested nicer
pub(crate) struct LogBuffer {
    buffer: String,
    lines: VecDeque<String>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            lines: VecDeque::new(),
        }
    }

    pub fn process(&mut self, new_data: &str) {
        self.buffer.push_str(new_data);

        while let Some(newline) = self.buffer.find('\n') {
            let line = self.buffer[..newline].to_string();
            self.buffer = self.buffer[newline + 1..].to_string();
            self.lines.push_back(line);
        }
    }

    /// deletes old lines and gives the current ones.
    pub fn take_lines(&mut self) -> VecDeque<String> {
        std::mem::take(&mut self.lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_line_processing() {
        let mut log_buffer = LogBuffer::new();

        log_buffer.process("Writing key KeySpec { destination: \"/et");
        log_buffer.process("c/keys/buildbot.aws.key\", user: \"buildbot\", group: \"buildbot-worker\", permissions: 384, length: 32, last: false, crc: 1370815231 }, 32 bytes of data");
        log_buffer.process("\n");
        log_buffer.process("xxx");
        log_buffer.process("xx_WIRE");
        log_buffer.process("_QUIT\n");
        let lines = log_buffer.take_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines.front().unwrap(),
            "Writing key KeySpec { destination: \"/etc/keys/buildbot.aws.key\", user: \"buildbot\", group: \"buildbot-worker\", permissions: 384, length: 32, last: false, crc: 1370815231 }, 32 bytes of data"
        );
        assert_eq!(lines.get(1).unwrap(), "xxxxx_WIRE_QUIT");

        // taking leaves none
        assert_eq!(log_buffer.lines.len(), 0);
    }
}
