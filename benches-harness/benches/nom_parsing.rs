use criterion::{black_box, criterion_group, criterion_main, Criterion};

use nmea::Error;
use nom::{
    bytes::complete::*, character::complete::*, combinator::*, number::complete::*,
    sequence::preceded,
};

#[allow(dead_code)]
struct MockBodData<'a> {
    bearing_true: Option<f32>,
    bearing_magnetic: Option<f32>,
    to_waypoint: Option<&'a str>,
    from_waypoint: Option<&'a str>,
}

static BOD_SENTENCE: &str = "097.0,T,103.2,M,POINTB,POINTA";
fn parsing_combinators_benchmark(c: &mut Criterion) {
    /*
    This benchmark was created to compare parsing strategies for handling commas during
    parsing of sentences.
     */
    let mut bench_group = c.benchmark_group("comma-separated parsing");

    bench_group.bench_function("let (i, ...) = preceded(char(','), ...)(i)?", |b| {
        b.iter(|| {
            _ = parse_bod_with_preceded(black_box(BOD_SENTENCE)).unwrap();
        })
    });

    bench_group.bench_function("let (i, _) = char(',')(i)?", |b| {
        b.iter(|| {
            _ = parse_bod_discard_comma(black_box(BOD_SENTENCE)).unwrap();
        })
    });

    let test_2 = "something,another";

    bench_group.bench_function("lite bench: preceded(char(','),...)", |b| {
        b.iter(|| {
            let (i, something) = take_until::<_, _, ()>(",")(test_2).unwrap();
            let (_, another) = preceded(char::<_, ()>(','), rest)(i).unwrap();
            black_box((something, another))
        })
    });

    bench_group.bench_function("lite bench: char(',')", |b| {
        b.iter(|| {
            let (i, something) = take_until::<_, _, ()>(",")(test_2).unwrap();
            let (i, _) = char::<_, ()>(',')(i).unwrap();
            let (_, another) = rest::<_, ()>(i).unwrap();
            black_box((something, another))
        })
    });
}

fn parse_bod_discard_comma(i: &str) -> Result<MockBodData, Error> {
    // 1. Bearing Degrees, True
    let (i, bearing_true) = opt(map_parser(take_until(","), float))(i)?;
    let (i, _) = char(',')(i)?;

    // 2. T = True
    let (i, _) = char('T')(i)?;
    let (i, _) = char(',')(i)?;

    // 3. Bearing Degrees, Magnetic
    let (i, bearing_magnetic) = opt(float)(i)?;
    let (i, _) = char(',')(i)?;

    // 4. M = Magnetic
    let (i, _) = char('M')(i)?;
    let (i, _) = char(',')(i)?;

    // 5. Destination Waypoint
    let (i, to_waypoint) = opt(is_not(",*"))(i)?;
    let (i, _) = char(',')(i)?;

    // 6. origin Waypoint
    let from_waypoint = opt(is_not("*"))(i)?.1;

    // 7. Checksum

    Ok(MockBodData {
        bearing_true,
        bearing_magnetic,
        to_waypoint,
        from_waypoint,
    })
}

fn parse_bod_with_preceded(i: &str) -> Result<MockBodData, Error> {
    // 1. Bearing Degrees, True
    let (i, bearing_true) = opt(map_parser(take_until(","), float))(i)?;

    // 2. T = True
    let (i, _) = preceded(char(','), char('T'))(i)?;

    // 3. Bearing Degrees, Magnetic
    let (i, bearing_magnetic) = preceded(char(','), opt(float))(i)?;

    // 4. M = Magnetic
    let (i, _) = preceded(char(','), char('M'))(i)?;

    // 5. Destination Waypoint
    let (i, to_waypoint) = preceded(char(','), opt(is_not(",*")))(i)?;

    // 6. origin Waypoint
    let from_waypoint = opt(preceded(char(','), is_not("*")))(i)?.1;

    // 7. Checksum

    Ok(MockBodData {
        bearing_true,
        bearing_magnetic,
        to_waypoint,
        from_waypoint,
    })
}

criterion_group!(benches, parsing_combinators_benchmark);
criterion_main!(benches);
