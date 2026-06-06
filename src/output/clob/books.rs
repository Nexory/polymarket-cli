use polymarket_client_sdk_v2::clob::types::response::{
    LastTradePriceResponse, LastTradesPricesResponse, OrderBookSummaryResponse,
};
use serde_json::json;
use tabled::settings::Style;
use tabled::{Table, Tabled};

use crate::output::{DASH, OutputFormat, truncate};

pub fn print_order_book(
    result: &OrderBookSummaryResponse,
    output: &OutputFormat,
) -> anyhow::Result<()> {
    match output {
        OutputFormat::Table => {
            println!("Market: {}", result.market);
            println!("Asset: {}", result.asset_id);
            println!(
                "Last Trade: {}",
                result
                    .last_trade_price
                    .map_or(DASH.into(), |p| p.to_string())
            );
            println!();

            #[derive(Tabled)]
            struct Row {
                #[tabled(rename = "Price")]
                price: String,
                #[tabled(rename = "Size")]
                size: String,
            }

            if result.bids.is_empty() {
                println!("No bids.");
            } else {
                println!("Bids:");
                let rows: Vec<Row> = result
                    .bids
                    .iter()
                    .rev()
                    .map(|o| Row {
                        price: o.price.to_string(),
                        size: o.size.to_string(),
                    })
                    .collect();
                let table = Table::new(rows).with(Style::rounded()).to_string();
                println!("{table}");
            }

            println!();

            if result.asks.is_empty() {
                println!("No asks.");
            } else {
                println!("Asks:");
                let rows: Vec<Row> = result
                    .asks
                    .iter()
                    .rev()
                    .map(|o| Row {
                        price: o.price.to_string(),
                        size: o.size.to_string(),
                    })
                    .collect();
                let table = Table::new(rows).with(Style::rounded()).to_string();
                println!("{table}");
            }
        }
        OutputFormat::Json => {
            crate::output::print_json(result)?;
        }
    }
    Ok(())
}

pub fn print_order_books(
    result: &[OrderBookSummaryResponse],
    output: &OutputFormat,
) -> anyhow::Result<()> {
    match output {
        OutputFormat::Table => {
            if result.is_empty() {
                println!("No order books found.");
                return Ok(());
            }
            for (i, book) in result.iter().enumerate() {
                if i > 0 {
                    println!();
                }
                print_order_book(book, output)?;
            }
        }
        OutputFormat::Json => {
            crate::output::print_json(result)?;
        }
    }
    Ok(())
}

pub fn print_last_trade(
    result: &LastTradePriceResponse,
    output: &OutputFormat,
) -> anyhow::Result<()> {
    match output {
        OutputFormat::Table => println!("Last Trade: {} ({})", result.price, result.side),
        OutputFormat::Json => {
            crate::output::print_json(&json!({
                "price": result.price.to_string(),
                "side": result.side.to_string(),
            }))?;
        }
    }
    Ok(())
}

pub fn print_last_trades_prices(
    result: &[LastTradesPricesResponse],
    output: &OutputFormat,
) -> anyhow::Result<()> {
    match output {
        OutputFormat::Table => {
            if result.is_empty() {
                println!("No last trade prices found.");
                return Ok(());
            }
            #[derive(Tabled)]
            struct Row {
                #[tabled(rename = "Token ID")]
                token_id: String,
                #[tabled(rename = "Price")]
                price: String,
                #[tabled(rename = "Side")]
                side: String,
            }
            let rows: Vec<Row> = result
                .iter()
                .map(|t| Row {
                    token_id: truncate(&t.token_id.to_string(), 20),
                    price: t.price.to_string(),
                    side: t.side.to_string(),
                })
                .collect();
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{table}");
        }
        OutputFormat::Json => {
            let data: Vec<_> = result
                .iter()
                .map(|t| {
                    json!({
                        "token_id": t.token_id.to_string(),
                        "price": t.price.to_string(),
                        "side": t.side.to_string(),
                    })
                })
                .collect();
            crate::output::print_json(&data)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use polymarket_client_sdk_v2::clob::types::TickSize;
    use polymarket_client_sdk_v2::clob::types::response::{OrderBookSummaryResponse, OrderSummary};
    use polymarket_client_sdk_v2::types::{B256, Decimal, U256};
    use rust_decimal_macros::dec;

    /// Build a minimal `OrderBookSummaryResponse` with caller-supplied bids and asks.
    /// The bids are provided in WORST-first order (ascending price) and asks in
    /// WORST-first order (descending price), which is what a naïve API response might
    /// contain and what the bug causes to be printed.
    fn make_book(
        bids: Vec<OrderSummary>,
        asks: Vec<OrderSummary>,
    ) -> OrderBookSummaryResponse {
        OrderBookSummaryResponse::builder()
            .market(B256::ZERO)
            .asset_id(U256::ZERO)
            .timestamp(Utc::now())
            .min_order_size(dec!(1))
            .neg_risk(false)
            .tick_size(TickSize::Hundredth)
            .bids(bids)
            .asks(asks)
            .build()
    }

    fn make_order(price: Decimal, size: Decimal) -> OrderSummary {
        OrderSummary::builder().price(price).size(size).build()
    }

    /// Returns the price strings in the order that `print_order_book` would render them,
    /// mirroring the exact `.iter().rev().map(|o| o.price.to_string())` chain used in the
    /// production function.
    fn rendered_bid_prices(book: &OrderBookSummaryResponse) -> Vec<String> {
        book.bids.iter().rev().map(|o| o.price.to_string()).collect()
    }

    fn rendered_ask_prices(book: &OrderBookSummaryResponse) -> Vec<String> {
        book.asks.iter().rev().map(|o| o.price.to_string()).collect()
    }

    /// The first bid shown must be the BEST bid (highest price = 0.50).
    /// The current code iterates in insertion order without sorting, so it
    /// renders 0.30 first — causing this test to FAIL on unpatched code.
    #[test]
    fn bids_rendered_best_price_first() {
        // Bids inserted worst-to-best (ascending price), as might come from the API.
        let book = make_book(
            vec![
                make_order(dec!(0.30), dec!(100)),
                make_order(dec!(0.40), dec!(100)),
                make_order(dec!(0.50), dec!(100)),
            ],
            vec![
                make_order(dec!(0.50), dec!(100)),
                make_order(dec!(0.60), dec!(100)),
                make_order(dec!(0.70), dec!(100)),
            ],
        );

        let bid_prices = rendered_bid_prices(&book);
        assert_eq!(
            bid_prices[0], "0.50",
            "first rendered bid must be the best (highest) bid price 0.50, got {} \
             — bids are printed in insertion order without sorting",
            bid_prices[0]
        );
    }

    /// The first ask shown must be the BEST ask (lowest price = 0.50).
    /// Current code iterates in insertion order — FAILS when asks are stored
    /// worst-first (descending price).
    #[test]
    fn asks_rendered_best_price_first() {
        // Asks inserted worst-to-best (descending price), as might come from the API.
        let book = make_book(
            vec![
                make_order(dec!(0.30), dec!(100)),
                make_order(dec!(0.40), dec!(100)),
                make_order(dec!(0.50), dec!(100)),
            ],
            vec![
                make_order(dec!(0.70), dec!(100)),
                make_order(dec!(0.60), dec!(100)),
                make_order(dec!(0.50), dec!(100)),
            ],
        );

        let ask_prices = rendered_ask_prices(&book);
        assert_eq!(
            ask_prices[0], "0.50",
            "first rendered ask must be the best (lowest) ask price 0.50, got {} \
             — asks are printed in insertion order without sorting",
            ask_prices[0]
        );
    }
}
