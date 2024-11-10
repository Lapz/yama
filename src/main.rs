use plotters::prelude::*;
use polars::{
    lazy::dsl::{col, lit, when},
    prelude::*,
};
use time::{
    macros::{date, format_description, offset},
    OffsetDateTime,
};
use yahoo_finance_api as yahoo;

macro_rules! struct_to_dataframe {
    ($input:expr, [$($field:ident),+]) => {
        {
            let len = $input.len().to_owned();

            // Extract the field values into separate vectors
            $(let mut $field = Vec::with_capacity(len);)*

            for e in $input.into_iter() {
                $($field.push(e.$field);)*
            }
            df! {
                $(stringify!($field) => $field,)*
            }
        }
    };
}

fn window_opts(size: usize) -> RollingOptionsFixedWindow {
    let mut config = RollingOptionsFixedWindow::default();
    config.window_size = size;

    config
}

fn plot_trading_signals(
    data: &DataFrame,
    output_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a drawing area
    let root = BitMapBackend::new(output_file, (1400, 700)).into_drawing_area();
    root.fill(&WHITE)?;

    // Get min and max values for scaling
    let close_min = data.column("close")?.f64()?.min().unwrap();
    let close_max = data.column("close")?.f64()?.max().unwrap();

    // Get date range
    let dates: Vec<i64> = data
        .column("timestamp")?
        .u64()?
        .into_iter()
        .map(|opt_dt| {
            let timestamp = opt_dt.unwrap() as i64;
            timestamp
        })
        .collect();

    println!("dates {:?}", dates.len());

    let date_min = dates.first().unwrap();
    let date_max = dates.last().unwrap();

    // Create chart
    let mut chart = ChartBuilder::on(&root)
        .caption("Moving Average Crossover Strategy", ("sans-serif", 30))
        .margin(10)
        .build_cartesian_2d(date_min.clone()..date_max.clone(), close_min..close_max)?;

    let date_format = format_description!("[year]-[month]");
    // Configure grid and labels
    chart
        .configure_mesh()
        .x_desc("Date")
        .y_desc("Price")
        .x_labels(20)
        .x_label_formatter(&|x| {
            // Convert months back to date string
            OffsetDateTime::from_unix_timestamp(*x)
                .unwrap()
                .format(date_format)
                .unwrap()
        })
        .y_label_formatter(&|y| format!("${:.2}", y))
        .draw()?;

    // Plot Close Price
    let close_data: Vec<(i64, f64)> = dates
        .iter()
        .zip(data.column("close")?.f64()?.into_iter())
        .map(|(dt, price)| (*dt, price.unwrap()))
        .collect();

    println!("close_data {:?}", close_data.len());

    chart
        .draw_series(LineSeries::new(close_data, &BLUE))?
        .label("Close Price");

    // Plot SMA_9
    let sma9_data: Vec<(i64, f64)> = dates
        .iter()
        .zip(data.column("SMA_9")?.f64()?.into_iter())
        .filter_map(|(dt, price)| price.map(|p| (*dt, p)))
        .collect();

    println!("sma_9 {:?}", sma9_data.len());
    chart
        .draw_series(LineSeries::new(sma9_data, &GREEN))?
        .label("9-day SMA");

    // Plot SMA_21
    let sma21_data: Vec<(i64, f64)> = dates
        .iter()
        .zip(data.column("SMA_21")?.f64()?.into_iter())
        .filter_map(|(dt, price)| price.map(|p| (*dt, p)))
        .collect();

    println!("sma_21 {:?}", sma21_data.len());
    chart
        .draw_series(LineSeries::new(sma21_data, &RED))?
        .label("21-day SMA");

    // Plot Buy Signals
    let buy_signals: Vec<(i64, f64)> = dates
        .iter()
        .zip(data.column("close")?.f64()?.into_iter())
        .zip(data.column("Signal")?.i32()?.into_iter())
        .filter_map(|((dt, price), signal)| {
            if signal == Some(1) {
                price.map(|p| (*dt, p))
            } else {
                None
            }
        })
        .collect();

    println!("buy_signals {:?}", buy_signals.len());
    chart
        .draw_series(PointSeries::of_element(
            buy_signals,
            5,
            &GREEN,
            &|c, s, st| {
                return EmptyElement::at(c)    // Position
                + TriangleMarker::new((0, -5), s, st.filled()); // Upward triangle
            },
        ))?
        .label("Buy Signal");

    // Plot Sell Signals
    let sell_signals: Vec<(i64, f64)> = dates
        .iter()
        .zip(data.column("close")?.f64()?.into_iter())
        .zip(data.column("Signal")?.i32()?.into_iter())
        .filter_map(|((dt, price), signal)| {
            if signal == Some(-1) {
                price.map(|p| (*dt, p))
            } else {
                None
            }
        })
        .collect();

    println!("sell_signals {:?}", sell_signals.len());

    chart
        .draw_series(PointSeries::of_element(
            sell_signals,
            5,
            &RED,
            &|c, s, st| {
                return EmptyElement::at(c)    // Position
                + TriangleMarker::new((0, 5), s, st.filled()); // Downward triangle
            },
        ))?
        .label("Sell Signal");

    // Draw legend
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}

fn plot_cumulative_returns(
    data: &DataFrame,
    output_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(output_file, (1400, 700)).into_drawing_area();
    root.fill(&WHITE)?;

    let cum_min = data.column("Cumulative_Return")?.f64()?.min().unwrap();
    let cum_max = data.column("Cumulative_Return")?.f64()?.max().unwrap();
    let range = cum_max - cum_min;
    let y_min = cum_min - (range * 0.05); // Add 5% padding
    let y_max = cum_max + (range * 0.05);

    // Get date range
    let dates: Vec<i64> = data
        .column("timestamp")?
        .u64()?
        .into_iter()
        .map(|opt_dt| {
            let timestamp = opt_dt.unwrap() as i64;
            timestamp
        })
        .collect();

    let date_min = dates.first().unwrap();
    let date_max = dates.last().unwrap();

    let mut chart = ChartBuilder::on(&root)
        .caption("Strategy Cumulative Return", ("sans-serif", 30))
        .build_cartesian_2d(date_min.clone()..date_max.clone(), y_min..y_max)?;

    let date_format = format_description!("[year]-[month]");

    chart
        .configure_mesh()
        .x_desc("Date")
        .x_labels(12)
        .x_label_formatter(&|x| {
            // Convert months back to date string
            OffsetDateTime::from_unix_timestamp(*x)
                .unwrap()
                .format(date_format)
                .unwrap()
        })
        .y_label_formatter(&|y| format!("{:.2}%", y * 100.0))
        .y_desc("Cumulative Return")
        .draw()?;

    let cumulative_returns: Vec<(i64, f64)> = dates
        .iter()
        .zip(data.column("Cumulative_Return")?.f64()?.into_iter())
        .filter_map(|(dt, price)| price.map(|p| (*dt, p)))
        .collect();

    chart
        .draw_series(LineSeries::new(cumulative_returns, &BLUE))?
        .label("Strategy Cumulative Return");

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}

fn calculate_max_drawdown(data: &DataFrame) -> Result<f64, PolarsError> {
    // Get the Cumulative_Return series
    let cumulative_returns = data.column("Cumulative_Return")?.f64()?;

    // Calculate running maximum manually
    let values: Vec<f64> = cumulative_returns
        .into_iter()
        .map(|x| x.unwrap_or(0.0))
        .collect();

    let mut cummax = Vec::with_capacity(values.len());
    let mut running_max = f64::MIN;

    for &value in values.iter() {
        running_max = running_max.max(value);
        cummax.push(running_max);
    }

    // Calculate drawdown
    let drawdowns: Vec<f64> = values
        .iter()
        .zip(cummax.iter())
        .map(|(&curr, &max)| (curr - max) / max)
        .collect();

    // Get the minimum drawdown (maximum loss)
    let max_drawdown = drawdowns.iter().fold(0.0_f64, |min, &x| min.min(x));

    Ok(max_drawdown * 100.0)
}

#[tokio::main]
async fn main() {
    let provider = yahoo::YahooConnector::new().unwrap();

    let ticker = "AAPL";

    let start_date = date!(2021 - 01 - 01).midnight().assume_offset(offset!(UTC));
    let end_date = date!(2022 - 01 - 01).midnight().assume_offset(offset!(UTC));

    let response = provider
        .get_quote_history(ticker, start_date, end_date)
        .await
        .unwrap()
        .quotes()
        .unwrap();

    let dataframe = struct_to_dataframe!(
        response,
        [timestamp, open, high, low, close, adjclose, volume]
    )
    .unwrap();

    let dataframe = dataframe
        .clone()
        .lazy()
        .with_columns([
            // Calculate short-term and long-term moving averages
            col("close").rolling_mean(window_opts(9)).alias("SMA_9"),
            col("close").rolling_mean(window_opts(21)).alias("SMA_21"),
            // Initialize the 'Signal' column with zeros
            lit(0).alias("Signal"),
        ])
        .collect()
        .unwrap();

    let dataframe = dataframe
        .clone()
        .lazy()
        .with_columns([when(col("SMA_9").gt(col("SMA_21")))
            .then(lit(1))
            .otherwise(lit(-1))
            .alias("Signal")])
        .collect()
        .unwrap();

    let result = plot_trading_signals(&dataframe, "trading_signals.png");

    if let Err(e) = result {
        eprintln!("Error creating plot: {}", e);
    }

    println!("{:?}", dataframe);

    // backtesting

    let dataframe = dataframe
        .clone()
        .lazy()
        .with_columns([col("close").pct_change(lit(1)).alias("Daily_Return")])
        .collect()
        .unwrap();

    let dataframe = dataframe
        .clone()
        .lazy()
        .with_columns(
            [(col("Signal").shift(lit(1)) * col("Daily_Return")).alias("Strategy_Return")],
        )
        .with_columns_seq([(lit(1) + col("Strategy_Return"))
            .cum_prod(false)
            .alias("Cumulative_Return")])
        .collect()
        .unwrap();

    let result = plot_cumulative_returns(&dataframe, "cumulative_returns.png");

    if let Err(e) = result {
        eprintln!("Error creating plot: {}", e);
    }

    let trading_days_per_year = 252.0;

    let annualized_return = dataframe
        .column("Strategy_Return")
        .unwrap()
        .f64()
        .unwrap()
        .mean()
        .unwrap()
        * trading_days_per_year;

    //  Calculate Annualized Volatility
    let annualized_volatility = dataframe
        .column("Strategy_Return")
        .unwrap()
        .f64()
        .unwrap()
        .std(0)
        .unwrap()
        * (trading_days_per_year.powf(0.5));
    // Calculate Sharpe Ratio

    let risk_free_rate = 0.01; // Assuming a risk-free rate of 1%
    let sharpe_ratio = (annualized_return - risk_free_rate) / annualized_volatility;

    // Calculate Maximum Drawdown

    let max_drawdown = calculate_max_drawdown(&dataframe).unwrap();

    println!("Annualized Return: {:.2}%", annualized_return);
    println!("Annualized Volatility: {:.2}", annualized_volatility);
    println!("Sharpe Ratio: {:.2}", sharpe_ratio);
    println!("Maximum Drawdown: {:.2}%", max_drawdown);
}
