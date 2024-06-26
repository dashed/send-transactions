use rayon::prelude::*;
use sentry::protocol::Request;
use sentry::protocol::{Event, Level};
use sentry::types::random_uuid;
use sentry::TransactionOrSpan;
use std::thread;
use std::time::Duration;

// cargo run --example performance-demo
fn main() {
    let _sentry = sentry::init((
        // "https://2f8e2ecf338b4d89ae932a124829eccb@o380891.ingest.sentry.io/5207397",
        "https://b52904e72a72c0ed8d3996cafe40d4af@o4507289623330816.ingest.us.sentry.io/4507352301240320",
        sentry::ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate: 1.0,
            // debug: true,
            ..Default::default()
        },
    ));

    (0..100_000_000).into_par_iter().for_each(|x: i64| {
        // if x % 10 == 0 {
        //     println!("loop: {}", x);
        // }
        generate_transactions(x);
        generate_transactions(x);
        generate_transactions(x);
        generate_transactions(x);
        generate_transactions(x);
        generate_transactions(x);
        generate_transactions(x);
        generate_transactions(x);
        generate_transactions(x);
        generate_transactions(x);
    });
}

fn generate_transactions(x: i64) {
    let transaction = sentry::start_transaction(sentry::TransactionContext::new(
        &format!("transaction {}", x),
        "root span",
    ));
    let tx_request = Request {
        url: Some("https://honk.beep".parse().unwrap()),
        method: Some("GET".to_string()),
        ..Request::default()
    };
    transaction.set_request(tx_request);
    sentry::configure_scope(|scope| scope.set_span(Some(transaction.clone().into())));

    main_span1();

    thread::sleep(Duration::from_millis(10));

    transaction.finish();
    sentry::configure_scope(|scope| scope.set_span(None));
}

fn main_span1() {
    wrap_in_span("span1", "", |_: &TransactionOrSpan| {
        thread::sleep(Duration::from_millis(5));

        let transaction_ctx = sentry::TransactionContext::continue_from_span(
            "background transaction",
            "root span",
            sentry::configure_scope(|scope| scope.get_span()),
        );
        thread::spawn(move || {
            let transaction = sentry::start_transaction(transaction_ctx);
            sentry::configure_scope(|scope| scope.set_span(Some(transaction.clone().into())));

            thread::sleep(Duration::from_millis(5));

            thread_span1();

            transaction.finish();
            sentry::configure_scope(|scope| scope.set_span(None));
        });
        thread::sleep(Duration::from_millis(10));

        main_span2()
    });
}

fn thread_span1() {
    wrap_in_span("span1", "", |_: &TransactionOrSpan| {
        thread::sleep(Duration::from_millis(20));
    });

    wrap_in_span(
        "db.query",
        "SELECT * FROM users WHERE id = %s",
        |span: &TransactionOrSpan| {
            let uuid = random_uuid();
            sentry::capture_event(Event {
                event_id: uuid,
                message: Some("Some error".into()),
                level: Level::Error,
                ..Default::default()
            });

            span.set_data("db.system", "postgresql".into());
            thread::sleep(Duration::from_millis(5));
        },
    );
}

fn main_span2() {
    wrap_in_span("span2", "", |_: &TransactionOrSpan| {
        sentry::capture_message(
            "A message that should have a trace context",
            sentry::Level::Info,
        );
        thread::sleep(Duration::from_millis(20));
    })
}

fn wrap_in_span<F, R>(op: &str, description: &str, f: F) -> R
where
    F: FnOnce(&TransactionOrSpan) -> R,
{
    let parent = sentry::configure_scope(|scope| scope.get_span());
    let span1: sentry::TransactionOrSpan = match &parent {
        Some(parent) => parent.start_child(op, description).into(),
        None => {
            let ctx = sentry::TransactionContext::new(description, op);
            sentry::start_transaction(ctx).into()
        }
    };
    let span_request = Request {
        url: Some("https://beep.beep".parse().unwrap()),
        method: Some("GET".to_string()),
        ..Request::default()
    };
    span1.set_request(span_request);
    sentry::configure_scope(|scope| scope.set_span(Some(span1.clone())));

    let rv = f(&span1);

    span1.finish();
    sentry::configure_scope(|scope| scope.set_span(parent));

    rv
}
