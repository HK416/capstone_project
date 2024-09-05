use std::thread;
use std::sync::Arc;
use std::time::Instant;
use mod_parallelism::collections::Queue;

const MAX_NUM: usize = 10_000;
const MAX_THREADS: usize = 16;
const NUM_TESTS: usize = 10_000_000;

enum History {
    Push(u32), 
    Pop(Option<Box<u32>>)
}


fn check_invalidation(historys: Vec<Vec<History>>, queue: Arc<Queue<Box<u32>>>) {
    let mut numbers: [i32; MAX_NUM + 1] = [0; MAX_NUM + 1];
    for history in historys {
        for record in history {
            match record {
                History::Push(val) => {
                    numbers[val as usize] += 1
                }, 
                History::Pop(result) => {
                    if let Some(val) = result {
                        numbers[*val as usize] -= 1;
                    }
                }
            }
        }
    }

    while let Some(val) = queue.pop() {
        numbers[*val as usize] -= 1;
    }

    for (number, count) in numbers.into_iter().enumerate() {
        if count == 0 {
            continue;
        } else if count < 0 {
            panic!("pop(dequeue) function is invalid! ({})", number);
        } else {
            panic!("push(enqueue) function is invalid! ({})", number);
        }
    }
}

fn validation_main(num_threads: usize, queue: Arc<Queue<Box<u32>>>) -> Vec<History> {
    let num_tests = NUM_TESTS / num_threads;
    let mut history = Vec::with_capacity(num_tests);

    for _ in 0..num_tests {
        if rand::random() {
            let mut val = rand::random();
            val = val % (MAX_NUM as u32 + 1);
            queue.push(Box::new(val));
            history.push(History::Push(val));
        } else {
            history.push(History::Pop(queue.pop()));
        }
    }

    return history;
}

fn thread_main(num_threads: usize, queue: Arc<Queue<u32>>) {
    let num_tests = NUM_TESTS / num_threads;
    for _ in 0..num_tests {
        if rand::random() {
            let mut val = rand::random();
            val = val % (MAX_NUM as u32 + 1);
            queue.push(val);
        } else {
            queue.pop();
        }
    }
}

fn main() {
    let mut num_threads = 1;
    while num_threads <= MAX_THREADS {
        print!("Benchmarking...");
        let start_t = Instant::now();
        let queue = Arc::new(Queue::new());
        let handles: Vec<_> = (0..num_threads).into_iter()
            .map(|_| {
                let queue_cloned = queue.clone();
                thread::spawn(move || thread_main(num_threads, queue_cloned))
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let exec_t = Instant::now()
            .saturating_duration_since(start_t)
            .as_millis();

        let mut count = 0;
        while let Some(value) = queue.pop() {
            if count == 16 { break; }
            print!("{}, ", value);
            count += 1;
        }
        print!("\n");
        print!("Threads = {}, Time: {}ms\n", num_threads, exec_t);
        drop(queue);

        print!("Checking consistency...");
        let queue = Arc::new(Queue::new());
        let handles: Vec<_> = (0..num_threads).into_iter()
            .map(|_| {
                let queue_cloned = queue.clone();
                thread::spawn(move || validation_main(num_threads, queue_cloned))
            })
            .collect();

        let mut historys = Vec::with_capacity(num_threads);
        for handle in handles {
            historys.push(handle.join().unwrap());
        }
        
        check_invalidation(historys, queue);
        print!("Okay!\n\n");

        num_threads *= 2;
    }
}
