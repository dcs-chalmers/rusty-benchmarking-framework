use crate:: {ConcurrentQueue, Handle};
use std::sync::Mutex;

pub struct BRingBuffer<T> {
    bounded_ringbuffer: Mutex<Vec<T>>,
    capacity: usize,
    head: Mutex<usize>,
    tail: Mutex<usize>,
    empty: Mutex<bool>
}

impl<T: Clone + Default> BRingBuffer<T> {
    pub fn new(capacity: usize) -> Self{
        BRingBuffer{
            bounded_ringbuffer: Mutex::new(vec![T::default(); capacity + 1]),
            capacity: capacity + 1,
            head: Mutex::new(0),
            tail: Mutex::new(0),
            empty: Mutex::new(true),
        }
    }

    pub fn pop(&self) -> Option<T>{
        let mut buf = self.bounded_ringbuffer.lock().unwrap();
        let mut tail = self.tail.lock().unwrap();
        let mut head = self.head.lock().unwrap();
        let mut empty = self.empty.lock().unwrap();

        if *empty{
            return None;
        }

        let item = buf[*head].clone();
        *head = (*head + 1) % self.capacity;

        *empty = *head == *tail;
        
        Some(item)
    }

    pub fn push(&self, item: T){
        let mut buf = self.bounded_ringbuffer.lock().unwrap();
        let mut tail = self.tail.lock().unwrap(); 
        let mut head = self.head.lock().unwrap();
        let mut empty = self.empty.lock().unwrap();

        buf[*tail] = item;
        *tail = (*tail + 1) % self.capacity;

        *empty = false;

        if *tail == *head{
            *head = (*head + 1) % self.capacity; //overwrite atm
        }

    }
}



pub struct BoundedRingBuffer<T>{
    pub brbuffer: BRingBuffer<T>
}

pub struct BRingBufferHandle<'a, T>{
    queue: &'a BoundedRingBuffer<T>
}


impl<T> Handle<T> for BRingBufferHandle<'_, T>{
    fn pop(&mut self) -> Option<T>{
        let mut buf = self.queue.brbuffer.bounded_ringbuffer.lock().unwrap();
        buf.pop()
    }
    fn push(&mut self, item: T){
        let mut buf = self.queue.brbuffer.bounded_ringbuffer.lock().unwrap();
        buf.push(item);
    } 
} 

impl <T>ConcurrentQueue<T> for BoundedRingBuffer<T>{
    fn register(&self) -> impl Handle<T>{
        BRingBufferHandle::<T> {
            queue: self,
        }
    }
}


#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn push_and_pop_single() {
        let buffer = BRingBuffer::new(3);
        buffer.push(5);
        assert_eq!(buffer.pop(), Some(5));
    }



    #[test]
    fn push_and_pop_multiple() {
        let buffer = BRingBuffer::new(3);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), None); // Buffer should be empty
        
    }

    #[test]
    fn overwrite_old_elements() {
        let buffer = BRingBuffer::new(3);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4); // Overwrites oldest element (1)
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn refill_buffer() {
        let buffer = BRingBuffer::new(2);
        buffer.push(1);
        buffer.push(2);
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), None); // Buffer empty

        // Refill after emptying
        buffer.push(3);
        buffer.push(4);
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn handle_full_buffer() {
        let buffer = BRingBuffer::new(2);
        buffer.push(10);
        buffer.push(20);
        buffer.push(30); // Overwrites 10
        assert_eq!(buffer.pop(), Some(20)); // 10 was overwritten
        assert_eq!(buffer.pop(), Some(30));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn empty_pop(){
        let buffer: BRingBuffer<i32> = BRingBuffer::new(5);
        assert_eq!(buffer.pop(), None);
    }


}