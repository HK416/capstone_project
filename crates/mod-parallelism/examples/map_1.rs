//! `SkipMap`의 빌림 성능 측정과 `SkipMap`의 결과가 유효한지 확인합니다.
//! 

use std::thread;
use std::sync::Arc;
use std::time::Instant;

use mod_parallelism::collections::SkipMap;

const MAX_THREADS: usize = 16;
const NUM_TESTS: usize = 10_000_000;

enum History {
    Increase, 
    Decrease, 
}



fn check_history(historys: Vec<Vec<History>>, map: Arc<SkipMap<u32, i32>>) {
    let mut number = 0;

    for historys in historys {
        for history in historys {
            match history {
                History::Increase => number += 1, 
                History::Decrease => number -= 1, 
            };
        }
    }

    assert_eq!(map.remove(&0).unwrap(), number, "ERROR. The two values are not equal.");
}

fn validation_main(num_threads: usize, map: Arc<SkipMap<u32, i32>>) -> Vec<History> {
    let num_tests = NUM_TESTS / num_threads;
    (0..num_tests).into_iter()
        .map(|_| {
            if rand::random() {
                *map.get_mut(&0).unwrap() += 1;
                History::Increase
            } else {
                *map.get_mut(&0).unwrap() -= 1;
                History::Decrease
            }
        })
        .collect()
}

fn thread_main(num_threads: usize, map: Arc<SkipMap<u32, i32>>) {
    let num_tests = NUM_TESTS / num_threads;

    for _ in 0..num_tests {
        if rand::random() {
            *map.get_mut(&0).unwrap() += 1;
        } else {
            *map.get_mut(&0).unwrap() -= 1;
        }
    }
}

fn main() {
    let mut num_threads = 1;
    while num_threads <= MAX_THREADS {
        print!("Benchmarking...");
        let start_t = Instant::now();
        let map = Arc::new(SkipMap::new());
        map.insert(0, 0);
        
        let handles: Vec<_> = (0..num_threads).into_iter()
            .map(|_| {
                let map_cloned = map.clone();
                thread::spawn(move || thread_main(num_threads, map_cloned))
            })
            .collect();
        
        for handle in handles {
            handle.join().unwrap();
        }

        let exec_t = Instant::now()
            .saturating_duration_since(start_t)
            .as_millis();

        print!("\n");
        print!("Threads = {}, Time: {}ms\n", num_threads, exec_t);
        drop(map);


        print!("Checking consistency...");
        let map = Arc::new(SkipMap::new());
        map.insert(0, 0);

        let handles: Vec<_> = (0..num_threads).into_iter()
            .map(|_| {
                let map_cloned = map.clone();
                thread::spawn(move || validation_main(num_threads, map_cloned))
            })
            .collect();

        let historys: Vec<_> = handles.into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        check_history(historys, map);
        print!("Okay!\n\n");

        num_threads *= 2;
    }
}
