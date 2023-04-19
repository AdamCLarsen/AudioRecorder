pub struct CircularBuffer<const N: usize, T> {
    data: [Option<T>; N],
    write_index: usize,
}

impl<const N: usize, T: Copy> CircularBuffer<N, T> {
    pub fn new() -> Self {
        CircularBuffer {
            data: [None; N],
            write_index: 0,
        }
    }

    pub fn put(&mut self, item: T) {
        self.data[self.write_index] = Some(item);
        self.write_index = (self.write_index + 1) % N;
    }

    pub fn read_fifo(&self) -> Vec<&T> {
        let mut fifo = Vec::new();
        let mut read_index = self.write_index;

        for _ in 0..N {
            if let Some(ref value) = self.data[read_index] {
                fifo.push(value);
            }

            read_index = (read_index + 1) % N;
        }

        fifo
    }

    pub fn read_unordered(&self) -> Vec<&T> {
        self.data.iter().filter_map(|item| item.as_ref()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::CircularBuffer;

    #[test]
    fn test_circular_buffer() {
        let mut buffer = CircularBuffer::<5, i32>::new();

        buffer.put(1);
        buffer.put(2);
        buffer.put(3);
        buffer.put(4);
        buffer.put(5);
        buffer.put(6); // overwrites first value

        let fifo_values = buffer.read_fifo();
        assert_eq!(fifo_values, [&6, &2, &3, &4, &5]);

        let unordered_values = buffer.read_unordered();
        let unordered_expected: Vec<&i32> = vec![&6, &2, &3, &4, &5];
        assert_eq!(unordered_values, unordered_expected);
    }
}
