use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use nucleo_matcher::pattern::{CaseMatching, Normalization};
use nucleo_matcher::Config;

use crate::Nucleo;

#[test]
fn active_injector_count() {
    let mut nucleo: Nucleo<()> = Nucleo::new(Config::DEFAULT, Arc::new(|| ()), Some(1), 1);
    assert_eq!(nucleo.active_injectors(), 0);
    let injector = nucleo.injector();
    assert_eq!(nucleo.active_injectors(), 1);
    let injector2 = nucleo.injector();
    assert_eq!(nucleo.active_injectors(), 2);
    drop(injector2);
    assert_eq!(nucleo.active_injectors(), 1);
    nucleo.restart(false);
    assert_eq!(nucleo.active_injectors(), 0);
    let injector3 = nucleo.injector();
    assert_eq!(nucleo.active_injectors(), 1);
    nucleo.tick(0);
    assert_eq!(nucleo.active_injectors(), 1);
    drop(injector);
    assert_eq!(nucleo.active_injectors(), 1);
    drop(injector3);
    assert_eq!(nucleo.active_injectors(), 0);
}

#[test]
fn tick_after_notify_sees_results() {
    let (tx, rx) = mpsc::channel();
    let notify = Arc::new(move || {
        let _ = tx.send(());
        // A slow callback keeps the worker busy right after notifying.
        thread::sleep(Duration::from_millis(50));
    });
    let mut nucleo: Nucleo<&str> = Nucleo::new(Config::DEFAULT, notify, Some(1), 1);
    let injector = nucleo.injector();
    // Enough items that matching outlasts try_lock's spin.
    let mut items = vec!["foo"; 10_000];
    items.extend(["bar", "baz"]);
    injector.extend(items.into_iter(), |item, cols| cols[0] = (*item).into());
    rx.recv().unwrap();
    nucleo
        .pattern
        .reparse(0, "ba", CaseMatching::Ignore, Normalization::Smart, false);
    let mut status = nucleo.tick(0);
    while status.running {
        rx.recv_timeout(Duration::from_secs(1))
            .expect("tick reported running but no notify followed");
        status = nucleo.tick(0);
    }
    assert_eq!(nucleo.snapshot().matched_item_count(), 2);
}
