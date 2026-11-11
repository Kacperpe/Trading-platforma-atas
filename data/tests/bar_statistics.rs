//! Behaviour tests for the bundled `bar-statistics.ind` table indicator.

use indicator_engine::{
    ChartFrame, FrameOutput, IndicatorConfig, IndicatorFrame, InstrumentContext, TimeOrTick,
    TradeAggregate, Value,
};

const SOURCE: &str = include_str!("../indicators/sources/bar-statistics.ind");
const HOUR_MS: i64 = 3_600_000;

fn frame(index: usize, time: i64, buy: f64, sell: f64, count: u64) -> IndicatorFrame {
    IndicatorFrame {
        index,
        time,
        basis: TimeOrTick::Time,
        instrument: InstrumentContext {
            min_tick: Some(0.1),
        },
        chart: ChartFrame {
            open: Some(100.0),
            high: Some(101.0),
            low: Some(99.0),
            close: Some(100.5),
            volume: Some(buy + sell),
            directional_volume_available: false,
            buy_qty: 0.0,
            sell_qty: 0.0,
            trade: Some(TradeAggregate {
                buy_qty: buy,
                sell_qty: sell,
                buy_count: count / 2,
                sell_count: count - count / 2,
            }),
        },
        open_interest: None,
        series_inputs: Vec::new().into(),
    }
}

fn number(output: &FrameOutput, id: &str) -> Option<f64> {
    match output.plots.iter().find(|plot| plot.id == id)?.value {
        Value::Float(value) => Some(value),
        Value::Int(value) => Some(value as f64),
        _ => None,
    }
}

fn run(frames: Vec<IndicatorFrame>) -> Vec<FrameOutput> {
    let compiled = indicator_engine::compile(SOURCE, "bar-statistics.ind")
        .expect("bar statistics should compile");
    let mut instance = compiled
        .instantiate(IndicatorConfig::default())
        .expect("bar statistics should instantiate");
    frames
        .into_iter()
        .map(|frame| {
            instance
                .evaluate_closed(frame)
                .expect("frame should evaluate")
        })
        .collect()
}

#[test]
fn per_bar_values() {
    let outputs = run(vec![frame(0, 0, 30.0, 10.0, 8)]);
    let bar = &outputs[0];
    assert_eq!(number(bar, "bar_stats.volume"), Some(40.0));
    assert_eq!(number(bar, "bar_stats.buy"), Some(30.0));
    assert_eq!(number(bar, "bar_stats.sell"), Some(10.0));
    assert_eq!(number(bar, "value"), Some(20.0));
    assert_eq!(number(bar, "bar_stats.delta_pct"), Some(50.0));
    assert_eq!(number(bar, "bar_stats.trades"), Some(8.0));
    assert_eq!(number(bar, "bar_stats.avg_trade"), Some(5.0));
}

#[test]
fn session_delta_accumulates_and_resets_at_utc_midnight() {
    // Hourly bars from 22:00 to 01:00 UTC across one midnight.
    let day = 24 * HOUR_MS;
    let outputs = run(vec![
        frame(0, day - 2 * HOUR_MS, 5.0, 2.0, 2), // +3
        frame(1, day - HOUR_MS, 1.0, 5.0, 2),     // -4
        frame(2, day, 6.0, 1.0, 2),               // +5, new session
        frame(3, day + HOUR_MS, 2.0, 0.0, 2),     // +2
    ]);
    let session: Vec<_> = outputs
        .iter()
        .map(|output| number(output, "bar_stats.cum_delta"))
        .collect();
    assert_eq!(session, [Some(3.0), Some(-1.0), Some(5.0), Some(7.0)]);
}
