use std::{
    cmp::Reverse,
    collections::HashMap,
    ops::AddAssign,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug, Default)]
struct Tracer {
    categories: HashMap<&'static str, Duration>,
    callstack_trace: HashMap<Vec<&'static str>, Duration>,
    callstack: Vec<&'static str>,
}

const DO_TRACE: bool = true;

static TRACER_INSTANCE: LazyLock<Mutex<Tracer>> = LazyLock::new(Default::default);

/// Traces the operation while automatically keeping track of the callstack
#[inline(always)]
pub fn trace_op<T>(label: &'static str, op: impl FnOnce() -> T) -> T {
    trace_callstack_push(label);
    let start = Instant::now();
    let res = op();
    let t = start.elapsed();
    trace_op_time(label, t);
    trace_callstack_pop(t);
    res
}

#[inline(always)]
pub fn trace_callstack_push(label: &'static str) {
    if !DO_TRACE {
        return;
    }
    let mut tracer = TRACER_INSTANCE.lock().unwrap();
    tracer.callstack.push(label);
}

#[inline(always)]
pub fn trace_callstack_pop(time: Duration) {
    if !DO_TRACE {
        return;
    }
    let mut tracer = TRACER_INSTANCE.lock().unwrap();
    let stack = tracer.callstack.clone();
    tracer
        .callstack_trace
        .entry(stack)
        .or_default()
        .add_assign(time);
    tracer.callstack.pop();
}

/// Recommended to use `trace_op` when possible, to avoid manually calling
/// []
#[inline(always)]
pub fn trace_op_time(label: &'static str, time: Duration) {
    if !DO_TRACE {
        return;
    }
    let mut tracer = TRACER_INSTANCE.lock().unwrap();
    tracer.categories.entry(label).or_default().add_assign(time);
}

fn print_cols<const N: usize>(cols: [&[String]; N], pad_after_cols: [usize; N]) {
    let num_rows = cols.iter().map(|col| col.len()).max().unwrap_or(0);
    let cols_max_width = cols
        .iter()
        .map(|col| col.iter().map(|entry| entry.len()).max().unwrap_or(0))
        .collect::<Vec<_>>();

    for row in 0..num_rows {
        for col in 0..N {
            let entry = cols[col].get(row).unwrap_or(const { &String::new() });
            let pad = cols_max_width[col] + pad_after_cols[0];
            print!("{entry:<pad$}");
        }
        println!();
    }
}

pub fn print_trace_time() {
    println!("=============");
    println!("Trace Results");
    let tracer = TRACER_INSTANCE.lock().unwrap();
    let mut categories = tracer
        .categories
        .iter()
        .map(|(a, b)| (*a, *b))
        .collect::<Vec<_>>();
    categories.sort_by_key(|(_, t)| Reverse(*t));

    if categories.is_empty() {
        println!("{{empty}}");
        println!("=============");
        return;
    }

    let mut col0 = vec![];
    let mut col1 = vec![];

    for (lbl, t) in categories.iter() {
        col0.push(format!("{lbl}"));
        col1.push(format!("{t:?}"));
    }

    print_cols([&col0, &col1], [4; _]);

    println!("=============");
    println!("Flamegraph");

    // Find unique

    //
    let mut callstack_trace = tracer
        .callstack_trace
        .clone()
        .into_iter()
        .collect::<Vec<_>>();
    // Lexographic sort ends up being the correct ordering
    callstack_trace.sort_by_cached_key(|(k, _)| k.clone());

    let mut col0 = vec![];
    let mut col1 = vec![];

    for (lbl, t) in callstack_trace.iter() {
        col0.push(format!("{}", lbl.join("::")));
        col1.push(format!("{t:?}"));
    }

    print_cols([&col0, &col1], [4; _]);

    println!("=============");
}

pub fn reset_trace() {
    let mut tracer = TRACER_INSTANCE.lock().unwrap();
    tracer.categories.clear();
}
