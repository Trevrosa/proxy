use criterion::{Criterion, criterion_group, criterion_main};
use proxy::rewrite_html_urls;

macro_rules! bench_url {
    ($name:ident, $url:literal) => {
        fn $name(c: &mut Criterion) {
            let html = reqwest::blocking::Client::new()
                .get($url)
                .send()
                .expect("should be able to get url")
                .text()
                .expect("should be able to get html text");

            c.bench_function(stringify!($name), |b| {
                b.iter_batched(
                    || html.clone(),
                    |html| rewrite_html_urls(html, $url, "localhost/proxy"),
                    criterion::BatchSize::SmallInput,
                )
            });
        }
    };
}

bench_url!(google, "https://www.google.com");
bench_url!(github, "https://github.com");

criterion_group!(benches, google, github);
criterion_main!(benches);
