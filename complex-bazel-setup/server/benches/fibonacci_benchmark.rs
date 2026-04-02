// Cargo.toml dependencies:
// [dependencies]
// criterion = { version = "0.5", features = ["html_reports"] }
//
// [[bench]]
// name = "fibonacci_benchmark"
// harness = false
// +nightly -Z unstable-options --frozen --quiet --release --timings=html RUST_LOG=debug %nextest %force RUST_BACKTRACE=full / --nocapture

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

// Recursive Fibonacci (inefficient)
fn fib_recursive(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fib_recursive(n - 1) + fib_recursive(n - 2),
    }
}

// Iterative Fibonacci (efficient)
fn fib_iterative(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut a = 0u64;
            let mut b = 1u64;
            for _ in 2..=n {
                let temp = a + b;
                a = b;
                b = temp;
            }
            b
        }
    }
}

// Memoized Fibonacci
fn fib_memoized(n: u32) -> u64 {
    fn fib_memo_helper(n: u32, memo: &mut Vec<Option<u64>>) -> u64 {
        if let Some(result) = memo[n as usize] {
            return result;
        }

        let result = match n {
            0 => 0,
            1 => 1,
            _ => fib_memo_helper(n - 1, memo) + fib_memo_helper(n - 2, memo),
        };

        memo[n as usize] = Some(result);
        result
    }

    let mut memo = vec![None; (n + 1) as usize];
    fib_memo_helper(n, &mut memo)
}

// Matrix multiplication Fibonacci (O(log n))
fn fib_matrix(n: u32) -> u64 {
    if n == 0 {
        return 0;
    }

    fn matrix_mult(a: [[u64; 2]; 2], b: [[u64; 2]; 2]) -> [[u64; 2]; 2] {
        [
            [
                a[0][0] * b[0][0] + a[0][1] * b[1][0],
                a[0][0] * b[0][1] + a[0][1] * b[1][1],
            ],
            [
                a[1][0] * b[0][0] + a[1][1] * b[1][0],
                a[1][0] * b[0][1] + a[1][1] * b[1][1],
            ],
        ]
    }

    fn matrix_pow(m: [[u64; 2]; 2], n: u32) -> [[u64; 2]; 2] {
        if n == 1 {
            return m;
        }

        let half = matrix_pow(m, n / 2);
        let half_squared = matrix_mult(half, half);

        if n % 2 == 0 {
            half_squared
        } else {
            matrix_mult(half_squared, m)
        }
    }

    let base = [[1, 1], [1, 0]];
    let result = matrix_pow(base, n);
    result[0][1]
}

fn benchmark_recursive(c: &mut Criterion) {
    let mut group = c.benchmark_group("fibonacci_recursive");

    for n in [10, 15, 20, 25, 30].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            b.iter(|| fib_recursive(black_box(n)));
        });
    }

    group.finish();
}

fn benchmark_iterative(c: &mut Criterion) {
    let mut group = c.benchmark_group("fibonacci_iterative");

    for n in [10, 20, 30, 40, 50, 60, 70, 80, 90, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            b.iter(|| fib_iterative(black_box(n)));
        });
    }

    group.finish();
}

fn benchmark_memoized(c: &mut Criterion) {
    let mut group = c.benchmark_group("fibonacci_memoized");

    for n in [10, 20, 30, 40, 50, 60, 70, 80, 90, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            b.iter(|| fib_memoized(black_box(n)));
        });
    }

    group.finish();
}

fn benchmark_matrix(c: &mut Criterion) {
    let mut group = c.benchmark_group("fibonacci_matrix");

    for n in [10, 20, 30, 40, 50, 60, 70, 80, 90, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            b.iter(|| fib_matrix(black_box(n)));
        });
    }

    group.finish();
}

fn benchmark_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("fibonacci_comparison");

    let n = 30;

    group.bench_function("recursive_30", |b| {
        b.iter(|| fib_recursive(black_box(n)));
    });

    group.bench_function("iterative_30", |b| {
        b.iter(|| fib_iterative(black_box(n)));
    });

    group.bench_function("memoized_30", |b| {
        b.iter(|| fib_memoized(black_box(n)));
    });

    group.bench_function("matrix_30", |b| {
        b.iter(|| fib_matrix(black_box(n)));
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_recursive,
    benchmark_iterative,
    benchmark_memoized,
    benchmark_matrix,
    benchmark_comparison
);
criterion_main!(benches);

// In src/lib.rs or src/main.rs for unit tests:
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_fibonacci_correctness() {
        let expected = vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55];

        for (i, &expected_val) in expected.iter().enumerate() {
            let n = i as u32;
            assert_eq!(
                fib_recursive(n),
                expected_val,
                "recursive failed for n={}",
                n
            );
            assert_eq!(
                fib_iterative(n),
                expected_val,
                "iterative failed for n={}",
                n
            );
            assert_eq!(fib_memoized(n), expected_val, "memoized failed for n={}", n);
            assert_eq!(fib_matrix(n), expected_val, "matrix failed for n={}", n);
        }
    }

    #[test]
    fn test_large_values() {
        // Test that all implementations give same results
        for n in 20..=40 {
            let iterative_result = fib_iterative(n);
            assert_eq!(fib_memoized(n), iterative_result);
            assert_eq!(fib_matrix(n), iterative_result);
        }
    }
}
