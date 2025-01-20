use lockfree;


pub fn start_benchmark() {
    println!("Hello world");
    let q: lockfree::queue::Queue<i32> = lockfree::queue::Queue::new();
}
