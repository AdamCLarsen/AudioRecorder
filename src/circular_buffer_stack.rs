#![allow(dead_code)]
pub struct CircularBufferStack<const N: usize, T> {
    data: [Option<T>; N],
    write_index: usize,
    loop_count: usize,
}

impl<const N: usize, T: Copy> CircularBufferStack<N, T> {
    pub fn new() -> Self {
        CircularBufferStack {
            data: [None; N],
            write_index: 0,
            loop_count: 0,
        }
    }

    pub fn put(&mut self, item: T) {
        self.data[self.write_index] = Some(item);
        self.write_index = (self.write_index + 1) % N;

        if self.write_index == 0 {
            self.loop_count += 1;
        }
    }

    pub fn is_full(&self) -> bool {
        self.loop_count > 0
    }

    pub fn clone(&self) -> Vec<&T> {
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

    pub fn clone_last_n(&self, last_n: usize) -> Vec<&T> {
        let mut fifo = Vec::new();
        let mut read_index = if self.write_index >= last_n { self.write_index - last_n } else { N - (last_n - self.write_index) };

        for _ in 0..last_n {
            if let Some(ref value) = self.data[read_index] {
                fifo.push(value);
            }

            read_index = (read_index + 1) % N;
        }

        fifo
    }

    pub fn clone_unordered(&self) -> Vec<&T> {
        self.data.iter().filter_map(|item| item.as_ref()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::CircularBufferStack;

    #[test]
    fn test_circular_buffer() {
        let mut buffer = CircularBufferStack::<5, i32>::new();

        buffer.put(1);
        buffer.put(2);
        buffer.put(3);
        buffer.put(4);
        buffer.put(5);
        buffer.put(6); // overwrites first value

        let fifo_values = buffer.clone();
        assert_eq!(fifo_values, [&2, &3, &4, &5, &6]);

        let fifo_last_n_values = buffer.clone_last_n(3);
        assert_eq!(fifo_last_n_values, [&4, &5, &6]);

        let unordered_values = buffer.clone_unordered();
        let unordered_expected: Vec<&i32> = vec![&6, &2, &3, &4, &5];
        assert_eq!(unordered_values, unordered_expected);
    }
}