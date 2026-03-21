use std::{
    cell::RefCell,
    cmp::Reverse,
    collections::HashMap,
    fs::File,
    ops::AddAssign,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

use inferno::flamegraph;
use parking_lot::ReentrantMutex;

#[derive(Debug, Default)]
struct Tracer {
    categories: HashMap<&'static str, Duration>,
    callstack_trace: HashMap<Vec<&'static str>, Duration>,
    callstack: Vec<&'static str>,
}

impl Tracer {
    #[inline(always)]
    fn trace_callstack_push(&mut self, label: &'static str) {
        if !DO_TRACE {
            return;
        }
        self.callstack.push(label);
    }

    #[inline(always)]
    fn trace_callstack_pop(&mut self, time: Duration) {
        if !DO_TRACE {
            return;
        }
        let stack = self.callstack.clone();
        self.callstack_trace
            .entry(stack)
            .or_default()
            .add_assign(time);
        self.callstack.pop();
    }

    #[inline(always)]
    fn trace_op_time(&mut self, label: &'static str, time: Duration) {
        if !DO_TRACE {
            return;
        }
        self.categories.entry(label).or_default().add_assign(time);
    }
}

const DO_TRACE: bool = true;

static TRACER_INSTANCE: LazyLock<ReentrantMutex<RefCell<Tracer>>> = LazyLock::new(Default::default);

/// Traces the operation while automatically keeping track of the callstack
#[inline(always)]
pub fn trace_op<T>(label: &'static str, op: impl FnOnce() -> T) -> T {
    if !DO_TRACE {
        return op();
    }

    let tracer = TRACER_INSTANCE.lock();

    tracer.borrow_mut().trace_callstack_push(label);
    let start = Instant::now();
    let res = op();
    let t = start.elapsed();
    let mut tracer = tracer.borrow_mut();
    tracer.trace_op_time(label, t);
    tracer.trace_callstack_pop(t);
    res
}

#[inline(always)]
pub fn trace_callstack_push(label: &'static str) {
    if !DO_TRACE {
        return;
    }
    TRACER_INSTANCE
        .lock()
        .borrow_mut()
        .trace_callstack_push(label)
}

#[inline(always)]
pub fn trace_callstack_pop(time: Duration) {
    if !DO_TRACE {
        return;
    }
    TRACER_INSTANCE
        .lock()
        .borrow_mut()
        .trace_callstack_pop(time)
}

/// Recommended to use `trace_op` when possible, to avoid manually manipulating the stack
#[inline(always)]
pub fn trace_op_time(label: &'static str, time: Duration) {
    if !DO_TRACE {
        return;
    }
    TRACER_INSTANCE
        .lock()
        .borrow_mut()
        .trace_op_time(label, time)
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

pub fn print_trace_time(opts: &PrintOpts) {
    let tracer = TRACER_INSTANCE.lock();
    let tracer = tracer.borrow();
    if opts.print_flat {
        println!("=============");
        println!("Trace Results");
        let mut categories = tracer
            .categories
            .iter()
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        categories.sort_by_key(|(_, t)| Reverse(*t));

        if categories.is_empty() {
            println!("{{empty}}");
            return;
        }

        let mut col0 = vec![];
        let mut col1 = vec![];

        for (lbl, t) in categories.iter() {
            col0.push(format!("{lbl}"));
            col1.push(format!("{t:?}"));
        }

        print_cols([&col0, &col1], [4; _]);
    }

    let mut callstack_trace = tracer
        .callstack_trace
        .clone()
        .into_iter()
        .collect::<Vec<_>>();
    // Lexographic sort is the correct ordering for printing the flamegraph
    callstack_trace.sort_by_cached_key(|(k, _)| k.clone());
    let callstack_trace = callstack_trace;

    if opts.print_flamegraph {
        println!("=============");
        println!("Flamegraph");

        let mut col0 = vec![];
        let mut col1 = vec![];

        for (lbl, t) in callstack_trace.iter() {
            col0.push(format!("{}", lbl.join("::")));
            col1.push(format!("{t:?}"));
        }
        print_cols([&col0, &col1], [4; _]);
    }

    if opts.write_flamegraph {
        let mut lines = vec![];

        for (lbl, t) in callstack_trace.iter() {
            let sample_ct = t.as_nanos();
            lines.push(format!("{} {}", lbl.join(";"), sample_ct));
        }

        let mut fg_opts = flamegraph::Options::default();

        let mut file = File::create("perf_tracer_flamegraph.svg").unwrap();

        flamegraph::from_lines(&mut fg_opts, lines.iter().map(|x| x.as_str()), &mut file).unwrap();
    }

    println!("=============");
}

#[derive(Debug)]
pub struct PrintOpts {
    pub print_flat: bool,
    pub write_flamegraph: bool,
    pub print_flamegraph: bool,
}

impl Default for PrintOpts {
    fn default() -> Self {
        Self {
            print_flat: true,
            write_flamegraph: true,
            print_flamegraph: false,
        }
    }
}

pub fn reset_trace() {
    let tracer = TRACER_INSTANCE.lock();
    let mut tracer = tracer.borrow_mut();
    tracer.categories.clear();
}
