use std::{collections::VecDeque, time::Duration};

use exchange::{
    SizeUnit, TickerInfo, Trade, UnixMs,
    unit::{Qty, qty::volume_size_unit},
};
use serde::{Deserialize, Serialize};

use crate::util::ok_or_default;

const TRADE_RETENTION_MS: u64 = 120_000;

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(deserialize_with = "ok_or_default", default)]
    pub ticker_display: TickerDisplay,
    #[serde(deserialize_with = "ok_or_default", default)]
    pub time_format: TimeFormat,
    pub trade_size_filter: f32,
    #[serde(default = "default_buffer_filter")]
    pub trade_retention: Duration,
    #[serde(deserialize_with = "ok_or_default", default)]
    pub stacked_bar: Option<StackedBar>,
    /// Smart Tape: merges consecutive prints into one row. `None` = raw tape.
    #[serde(deserialize_with = "ok_or_default", default)]
    pub smart_tape: Option<SmartTape>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ticker_display: TickerDisplay::default(),
            time_format: TimeFormat::default(),
            trade_size_filter: 0.0,
            trade_retention: Duration::from_millis(TRADE_RETENTION_MS),
            stacked_bar: StackedBar::Compact(StackedBarRatio::default()).into(),
            smart_tape: None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TickerDisplay {
    #[default]
    Icon,
    IconAndTicker,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeFormat {
    #[default]
    MinutesSecondsMillis,
    HoursMinutesSecondsMillis,
    HoursMinutesSeconds,
}

impl TimeFormat {
    pub const ALL: [Self; 3] = [
        Self::MinutesSecondsMillis,
        Self::HoursMinutesSecondsMillis,
        Self::HoursMinutesSeconds,
    ];

    pub fn pattern(self) -> &'static str {
        match self {
            Self::MinutesSecondsMillis => "%M:%S.%3f",
            Self::HoursMinutesSecondsMillis => "%H:%M:%S.%3f",
            Self::HoursMinutesSeconds => "%H:%M:%S",
        }
    }
}

impl std::fmt::Display for TimeFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MinutesSecondsMillis => "mm:ss.ms",
            Self::HoursMinutesSecondsMillis => "hh:mm:ss.ms",
            Self::HoursMinutesSeconds => "hh:mm:ss",
        })
    }
}

fn default_buffer_filter() -> Duration {
    Duration::from_millis(TRADE_RETENTION_MS)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Copy)]
pub enum StackedBar {
    Compact(StackedBarRatio),
    Full(StackedBarRatio),
}

impl StackedBar {
    pub fn ratio(self) -> StackedBarRatio {
        match self {
            StackedBar::Compact(r) | StackedBar::Full(r) => r,
        }
    }

    pub fn with_ratio(self, r: StackedBarRatio) -> Self {
        match self {
            StackedBar::Compact(_) => StackedBar::Compact(r),
            StackedBar::Full(_) => StackedBar::Full(r),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default, Copy)]
pub enum StackedBarRatio {
    Count,
    #[default]
    Volume,
    AverageSize,
}

impl std::fmt::Display for StackedBarRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackedBarRatio::Count => write!(f, "Count"),
            StackedBarRatio::AverageSize => write!(f, "Average trade size"),
            StackedBarRatio::Volume => write!(f, "Volume"),
        }
    }
}

impl StackedBarRatio {
    pub const ALL: [StackedBarRatio; 3] = [
        StackedBarRatio::Count,
        StackedBarRatio::Volume,
        StackedBarRatio::AverageSize,
    ];
}

/// Smart Tape settings.
///
/// Consecutive prints of the same ticker and aggressor side whose timestamps
/// fall within `window_ms` of the first print in the group are merged into a
/// single row. With `window_ms == 0` only prints sharing an identical
/// timestamp are merged (typically the fills of one aggressive order).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartTape {
    pub window_ms: u32,
}

impl SmartTape {
    pub const MAX_WINDOW_MS: u32 = 1_000;
}


#[derive(Clone, Copy)]
pub struct TapeTrade {
    trade: Trade,
    ticker_info: TickerInfo,
    notional: Qty,
    /// Number of exchange prints merged into this row (1 for a raw print).
    fills: u32,
    /// Lowest and highest fill price, used to show how far an order swept.
    price_low: exchange::unit::Price,
    price_high: exchange::unit::Price,
    /// Timestamp of the first print in the group (`trade.time` is the last).
    first_time: UnixMs,
}

impl std::ops::Deref for TapeTrade {
    type Target = Trade;

    fn deref(&self) -> &Trade {
        &self.trade
    }
}

impl TapeTrade {
    pub fn new(ticker_info: TickerInfo, trade: Trade, size_unit: SizeUnit) -> Self {
        let notional = Qty::from_f64(ticker_info.market_type().qty_in_quote_value(
            trade.qty,
            trade.price,
            size_unit == SizeUnit::Quote,
        ));
        Self {
            trade,
            ticker_info,
            notional,
            fills: 1,
            price_low: trade.price,
            price_high: trade.price,
            first_time: trade.time,
        }
    }

    pub fn fills(&self) -> u32 {
        self.fills
    }

    /// Number of distinct price ticks the merged prints spanned (1 = single price).
    pub fn swept_ticks(&self) -> i64 {
        let tick = exchange::unit::Price::min_tick_units(self.ticker_info.min_ticksize).max(1);
        (self.price_high.units - self.price_low.units) / tick + 1
    }

    fn can_absorb(&self, next: &Self, window_ms: u32) -> bool {
        self.ticker_info.ticker == next.ticker_info.ticker
            && self.is_sell == next.is_sell
            && next.time >= self.first_time
            && next.time.as_u64() - self.first_time.as_u64() <= u64::from(window_ms)
    }

    fn absorb(&mut self, next: &Self) {
        self.trade.qty += next.qty;
        self.notional += next.notional;
        self.trade.price = next.price;
        self.trade.time = self.trade.time.max(next.time);
        self.price_low = self.price_low.min(next.price);
        self.price_high = self.price_high.max(next.price);
        self.fills = self.fills.saturating_add(next.fills);
    }

    /// Appends a time-ordered print to Smart Tape rows, merging it into the
    /// last row when it belongs to the same aggressive order.
    /// Returns `true` when the print was merged instead of pushed.
    pub fn coalesce_into(rows: &mut VecDeque<Self>, trade: Self, settings: SmartTape) -> bool {
        if let Some(last) = rows.back_mut()
            && last.can_absorb(&trade, settings.window_ms)
        {
            last.absorb(&trade);
            return true;
        }
        rows.push_back(trade);
        false
    }

    /// Rebuilds Smart Tape rows from a time-ordered raw tape.
    pub fn build_smart_rows<'a>(
        raw: impl IntoIterator<Item = &'a Self>,
        settings: SmartTape,
    ) -> VecDeque<Self> {
        let mut rows = VecDeque::new();
        for trade in raw {
            Self::coalesce_into(&mut rows, *trade, settings);
        }
        rows
    }

    pub fn ticker_info(&self) -> TickerInfo {
        self.ticker_info
    }

    pub fn quantity_text(&self, multiple_tickers: bool) -> String {
        let base = format!(
            "{}{}",
            self.quantity_prefix(multiple_tickers, volume_size_unit()),
            crate::util::abbr_large_numbers(self.quantity(multiple_tickers).to_f64())
        );
        if self.fills > 1 {
            format!("{base} ×{}", self.fills)
        } else {
            base
        }
    }

    pub fn quantity(&self, multiple_tickers: bool) -> Qty {
        if multiple_tickers {
            self.notional
        } else {
            self.qty
        }
    }

    pub fn passes_filter(&self, threshold: f32) -> bool {
        self.notional.to_f32_lossy() >= threshold
    }

    pub fn quantity_prefix(&self, multiple_tickers: bool, size_unit: SizeUnit) -> &'static str {
        if multiple_tickers
            || size_unit == SizeUnit::Quote
            || self.ticker_info.market_type() == exchange::adapter::MarketKind::InversePerps
        {
            "$"
        } else {
            ""
        }
    }

    pub fn merge_into(target: &mut VecDeque<Self>, mut incoming: Vec<Self>) {
        if incoming.is_empty() {
            return;
        }
        incoming.sort_by_key(|entry| entry.time);
        if target
            .back()
            .is_none_or(|last| last.time <= incoming[0].time)
        {
            target.extend(incoming);
            return;
        }
        let merge_start = target.partition_point(|entry| entry.time <= incoming[0].time);
        let mut previous = target.split_off(merge_start);
        for entry in incoming {
            while previous.front().is_some_and(|old| old.time <= entry.time) {
                target.push_back(previous.pop_front().unwrap());
            }
            target.push_back(entry);
        }
        target.append(&mut previous);
    }
}

pub fn prune_trades(trades: &mut VecDeque<TapeTrade>, retention: Duration, now: UnixMs) -> bool {
    let retention_ms = retention.as_millis() as u64;
    let prune_slack_ms = retention_ms / 10;
    let retention_cutoff = now.saturating_sub(retention_ms);
    let slack_cutoff = now.saturating_sub(retention_ms.saturating_add(prune_slack_ms));

    if trades
        .front()
        .is_none_or(|oldest| oldest.time >= slack_cutoff)
    {
        return false;
    }

    let previous_len = trades.len();
    while trades
        .front()
        .is_some_and(|trade| trade.time < retention_cutoff)
    {
        trades.pop_front();
    }
    trades.len() != previous_len
}

#[derive(Debug, Clone, Copy)]
pub enum HistAggValues {
    Count { buy: u64, sell: u64 },
    Qty { buy: Qty, sell: Qty },
}

#[derive(Default)]
pub struct HistAgg {
    buy_count: u64,
    sell_count: u64,
    buy_sum: Qty,
    sell_sum: Qty,
}

impl HistAgg {
    fn average_qty(sum: Qty, count: u64) -> Qty {
        if count == 0 {
            return Qty::ZERO;
        }

        let c = count.min(i64::MAX as u64) as i64;
        let half = c / 2;
        let rounded = if sum.units >= 0 {
            sum.units.saturating_add(half).div_euclid(c)
        } else {
            sum.units.saturating_sub(half).div_euclid(c)
        };

        Qty::from_units(rounded)
    }

    pub fn add(&mut self, is_sell: bool, qty: Qty) {
        if is_sell {
            self.sell_count += 1;
            self.sell_sum = Qty::from_units(self.sell_sum.units.saturating_add(qty.units));
        } else {
            self.buy_count += 1;
            self.buy_sum = Qty::from_units(self.buy_sum.units.saturating_add(qty.units));
        }
    }

    pub fn values_for(&self, ratio_kind: StackedBarRatio) -> Option<HistAggValues> {
        match ratio_kind {
            StackedBarRatio::Count => {
                let buy = self.buy_count;
                let sell = self.sell_count;
                let total = buy.saturating_add(sell);

                if total == 0 {
                    return None;
                }

                Some(HistAggValues::Count { buy, sell })
            }
            StackedBarRatio::Volume => {
                let buy = self.buy_sum.to_f32_lossy();
                let sell = self.sell_sum.to_f32_lossy();
                let total = buy + sell;

                if total <= 0.0 {
                    return None;
                }

                Some(HistAggValues::Qty {
                    buy: self.buy_sum,
                    sell: self.sell_sum,
                })
            }
            StackedBarRatio::AverageSize => {
                let buy_avg = Self::average_qty(self.buy_sum, self.buy_count);
                let sell_avg = Self::average_qty(self.sell_sum, self.sell_count);

                let denom = buy_avg.units.saturating_add(sell_avg.units);
                if denom <= 0 {
                    return None;
                }

                Some(HistAggValues::Qty {
                    buy: buy_avg,
                    sell: sell_avg,
                })
            }
        }
    }
}

#[cfg(test)]
mod smart_tape_tests {
    use super::*;
    use exchange::unit::Price;

    fn info(symbol: &str) -> TickerInfo {
        TickerInfo::new(
            exchange::Ticker::new(symbol, exchange::adapter::Exchange::BinanceLinear),
            0.1,
            0.001,
            None,
        )
    }

    fn print(info: TickerInfo, time: u64, is_sell: bool, price: f64, qty: f64) -> TapeTrade {
        TapeTrade::new(
            info,
            Trade {
                time: time.into(),
                is_sell,
                price: Price::from_f64(price),
                qty: Qty::from_f64(qty),
            },
            SizeUnit::Base,
        )
    }

    #[test]
    fn merges_same_timestamp_same_side() {
        let btc = info("BTCUSDT");
        let raw = [
            print(btc, 1_000, false, 100.0, 1.0),
            print(btc, 1_000, false, 100.1, 2.0),
            print(btc, 1_000, false, 100.3, 0.5),
        ];
        let rows = TapeTrade::build_smart_rows(raw.iter(), SmartTape::default());
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(row.fills(), 3);
        assert_eq!(row.qty, Qty::from_f64(3.5));
        assert_eq!(row.price, Price::from_f64(100.3));
        assert_eq!(row.swept_ticks(), 4);
    }

    #[test]
    fn splits_on_side_ticker_and_time() {
        let btc = info("BTCUSDT");
        let eth = info("ETHUSDT");
        let raw = [
            print(btc, 1_000, false, 100.0, 1.0),
            print(btc, 1_000, true, 100.0, 1.0), // other side
            print(eth, 1_000, true, 100.0, 1.0), // other ticker
            print(eth, 1_001, true, 100.0, 1.0), // other timestamp (window 0)
        ];
        let rows = TapeTrade::build_smart_rows(raw.iter(), SmartTape::default());
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|row| row.fills() == 1));
    }

    #[test]
    fn window_is_measured_from_first_print() {
        let btc = info("BTCUSDT");
        let raw = [
            print(btc, 1_000, false, 100.0, 1.0),
            print(btc, 1_040, false, 100.0, 1.0),
            print(btc, 1_050, false, 100.0, 1.0),
            print(btc, 1_051, false, 100.0, 1.0), // 51 ms after the first -> new row
        ];
        let rows = TapeTrade::build_smart_rows(raw.iter(), SmartTape { window_ms: 50 });
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fills(), 3);
        assert_eq!(rows[0].time, 1_050.into());
        assert_eq!(rows[1].fills(), 1);
    }

    #[test]
    fn aggregated_rows_pass_size_filter_by_total_notional() {
        let btc = info("BTCUSDT");
        let raw = [
            print(btc, 1_000, false, 100.0, 3.0),
            print(btc, 1_000, false, 100.0, 3.0),
        ];
        assert!(!raw[0].passes_filter(500.0));
        let rows = TapeTrade::build_smart_rows(raw.iter(), SmartTape::default());
        assert!(rows[0].passes_filter(500.0));
        assert!(rows[0].quantity_text(false).ends_with(" ×2"));
    }
}
