//

use std::thread;

extern crate crossbeam;
use crossbeam::channel::bounded;

fn main() {
    let (sender, receiver) = bounded::<i64>(10);

    let mut producers = Vec::new();
    let mut consumers = Vec::new();

    for _ in 0..3 {
        let input = receiver.clone();
        let consumer = thread::spawn(move || {
            let mut sum = 0;
            loop {
                let task = input.recv().unwrap();
                if task == 0 {
                    return sum;
                } else {
                    sum += task;
                }
            }
        });
        consumers.push(consumer);
        let output = sender.clone();
        let producer = thread::spawn(move || {
            for _ in 0..1024 {
                for i in -100..=100 {
                    if i == 0 {
                        continue;
                    }
                    output.send(i).unwrap();
                }
            }
            output.send(0).unwrap();
        });
        producers.push(producer);
    }

    let mut sum = Vec::new();
    for producer in producers.into_iter() {
        producer.join().unwrap();
    }
    for consumer in consumers.into_iter() {
        sum.push(consumer.join().unwrap());
    }

    println!(
        "{} + {} + {} = {}",
        sum[0],
        sum[1],
        sum[2],
        sum[0] + sum[1] + sum[2]
    );
}
