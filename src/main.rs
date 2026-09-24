use std::sync::mpsc ;
use std::thread ;

fn main() {
    let (tr, rs)  = mpsc::channel::<i32>() ;
    let mut hands = vec![] ;

    for i in 0..100 {
        hands.push(
        thread::spawn({ 
                let tr = tr.clone() ;
                move || {
                    tr.send(i).unwrap() ;
                }
            })
        ) ;
    }

    drop(tr);   // Без drop(tr) программа зависнет навсегда.

    // ожидание завершения всех потоков
    for h in hands {
        h.join().unwrap() ;
    }

    // Цикл for r in rs завершается, когда все клоны ts и он сам 
    // будут уничтожены.
    for r in rs {
        println!("From thread N: {r}") ;
    }
}
