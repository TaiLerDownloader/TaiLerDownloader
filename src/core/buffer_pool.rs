use crossbeam::queue::SegQueue;

pub struct BufferPool {
    pool: SegQueue<Vec<u8>>,
    chunk_size: usize,
}

impl BufferPool {
    pub fn new(chunk_size: usize, initial_count: usize) -> Self {
        let pool = SegQueue::new();
        for _ in 0..initial_count {
            pool.push(vec![0u8; chunk_size]);
        }
        BufferPool { pool, chunk_size }
    }

    pub fn get(&self) -> Vec<u8> {
        self.pool
            .pop()
            .unwrap_or_else(|| vec![0u8; self.chunk_size])
    }

    pub fn put(&self, buffer: Vec<u8>) {
        if buffer.len() == self.chunk_size {
            self.pool.push(buffer);
        }
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

pub static HTTP_BUFFER_POOL: once_cell::sync::Lazy<BufferPool> =
    once_cell::sync::Lazy::new(|| BufferPool::new(64 * 1024, 256));

pub static FILE_BUFFER_POOL: once_cell::sync::Lazy<BufferPool> =
    once_cell::sync::Lazy::new(|| BufferPool::new(64 * 1024, 128));

#[inline]
pub fn get_http_buffer() -> Vec<u8> {
    HTTP_BUFFER_POOL.get()
}

#[inline]
pub fn put_http_buffer(buffer: Vec<u8>) {
    HTTP_BUFFER_POOL.put(buffer);
}

#[inline]
pub fn get_file_buffer() -> Vec<u8> {
    FILE_BUFFER_POOL.get()
}

#[inline]
pub fn put_file_buffer(buffer: Vec<u8>) {
    FILE_BUFFER_POOL.put(buffer);
}
