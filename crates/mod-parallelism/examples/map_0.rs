//! `SkipMap`의 삽입/삭제 성능 측정과 `SkipMap`의 결과가 유효한지 확인합니다.
//! 

use std::thread;
use std::sync::Arc;
use std::time::Instant;

use mod_parallelism::collections::SkipMap;

const MAX_NUM: usize = 100_000;
const MAX_THREADS: usize = 16;
const NUM_TESTS: usize = 10_000_000;

enum History {
    Insert { val: u32, result: Option<u32> }, 
    Remove { val: u32, result: Option<u32> }, 
}



fn check_history(historys: Vec<Vec<History>>, map: Arc<SkipMap<u32, u32>>) {
    let mut survive = [0; MAX_NUM + 1];

    for historys in historys {
        for history in historys {
            match history {
                History::Insert { val, result } => {
                    if result.is_none() {
                        survive[val as usize] += 1;
                    }
                },
                History::Remove { val, result } => {
                    if result.is_some() {
                        survive[val as usize] -= 1;
                    }
                }, 
            };
        }
    }

    for (num, cnt) in survive.into_iter().enumerate() {
        if cnt < 0 {
            panic!("ERROR. The value {} removed while it is not in the set.", num);
        } else if cnt > 1 {
            panic!("ERROR. The value {} is added while the set already have it.", num);
        } else if cnt == 0 && map.contains_key(&(num as u32)) {
            panic!("ERROR. The value {} should not exists.", num);
        } else if cnt == 1 && !map.contains_key(&(num as u32)) {
            panic!("ERROR. The value {} should exists.", num);
        }
    }
}

fn validation_main(num_threads: usize, map: Arc<SkipMap<u32, u32>>) -> Vec<History> {
    let num_tests = NUM_TESTS / num_threads;
    (0..num_tests).into_iter()
        .map(|_| {
            let mut val = rand::random();
            val = val % (MAX_NUM as u32 + 1);
            if rand::random() {
                History::Insert { val, result: map.insert(val, val) }
            } else {
                History::Remove { val, result: map.remove(&val) }
            }
        })
        .collect()
}

fn thread_main(num_threads: usize, map: Arc<SkipMap<u32, u32>>) {
    let num_tests = NUM_TESTS / num_threads;

    for _ in 0..num_tests {
        let mut val = rand::random();
        val = val % (MAX_NUM as u32 + 1);

        if rand::random() {
            map.insert(val, val);
        } else {
            map.remove(&val);
        }
    }
}

fn main() {
    let mut num_threads = 1;
    while num_threads <= MAX_THREADS {
        print!("Benchmarking...");
        let start_t = Instant::now();
        let map = Arc::new(SkipMap::new());
        
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
