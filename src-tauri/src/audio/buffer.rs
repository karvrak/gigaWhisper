//! Ring Buffer
//!
//! Efficient circular buffer for audio samples.

/// Ring buffer for audio samples
pub struct RingBuffer {
    data: Vec<f32>,
    write_pos: usize,
    read_pos: usize,
    count: usize,
}

impl RingBuffer {
    /// Create a new ring buffer with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0.0; capacity],
            write_pos: 0,
            read_pos: 0,
            count: 0,
        }
    }

    /// Write samples to the buffer
    pub fn write(&mut self, samples: &[f32]) {
        for &sample in samples {
            self.data[self.write_pos] = sample;
            self.write_pos = (self.write_pos + 1) % self.data.len();

            if self.count < self.data.len() {
                self.count += 1;
            } else {
                // Buffer full, advance read position
                self.read_pos = (self.read_pos + 1) % self.data.len();
            }
        }
    }

    /// Read all available samples (non-destructive)
    pub fn read_all(&self) -> Vec<f32> {
        let mut result = Vec::with_capacity(self.count);

        if self.count == 0 {
            return result;
        }

        let mut pos = self.read_pos;
        for _ in 0..self.count {
            result.push(self.data[pos]);
            pos = (pos + 1) % self.data.len();
        }

        result
    }

    /// Drain all samples (destructive read)
    pub fn drain(&mut self) -> Vec<f32> {
        let result = self.read_all();
        self.clear();
        result
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.write_pos = 0;
        self.read_pos = 0;
        self.count = 0;
    }

    /// Get number of samples in buffer
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_read() {
        let mut buffer = RingBuffer::new(10);

        buffer.write(&[1.0, 2.0, 3.0]);
        assert_eq!(buffer.len(), 3);

        let samples = buffer.read_all();
        assert_eq!(samples, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_overflow() {
        let mut buffer = RingBuffer::new(5);

        buffer.write(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        assert_eq!(buffer.len(), 5);

        let samples = buffer.read_all();
        assert_eq!(samples, vec![3.0, 4.0, 5.0, 6.0, 7.0]);
    }

    #[test]
    fn test_drain() {
        let mut buffer = RingBuffer::new(10);

        buffer.write(&[1.0, 2.0, 3.0]);
        let samples = buffer.drain();

        assert_eq!(samples, vec![1.0, 2.0, 3.0]);
        assert!(buffer.is_empty());
    }
}
