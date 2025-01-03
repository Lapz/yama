import yfinance as yf
import polars as pl
import matplotlib.pyplot as plt
import select


ticker = "AAPL"  # Apple stock
start_date = "2021-01-01"
end_date = "2022-01-01"


def visulize_strategy(data):
    plt.figure(figsize=(14, 7))
    plt.plot(data["Date"], data["('Close', 'AAPL')"], label="Close Price", color="blue")
    plt.plot(
        data["Date"], data["SMA_9"], label="9-day SMA", color="green", linestyle="--"
    )
    plt.plot(
        data["Date"], data["SMA_9"], label="21-day SMA", color="red", linestyle="--"
    )
    # Highlighting Buy and Sell signals on the chart
    buy_signals = data.select()
    sell_signals = data[data["Signal"] == -1]
    plt.scatter(
        buy_signals.index,
        buy_signals["Close"],
        marker="^",
        color="green",
        label="Buy Signal",
        alpha=1,
    )
    plt.scatter(
        sell_signals.index,
        sell_signals["Close"],
        marker="v",
        color="red",
        label="Sell Signal",
        alpha=1,
    )
    plt.title("Moving Average Crossover Strategy")
    plt.xlabel("Date")
    plt.ylabel("Price")
    plt.legend()
    plt.grid(True)
    plt.show()


def main():
    # Downloading historical data using yfinance
    #
    #
    original_data = yf.download(ticker, start=start_date, end=end_date)

    if original_data is None:
        return "Fialed to download data"

    data = pl.from_pandas(original_data, include_index=True)

    original_data["SMA_9"] = (
        original_data["Close"].rolling(window=9).mean()
    )  # 10-day SMA
    original_data["SMA_21"] = (
        original_data["Close"].rolling(window=21).mean()
    )  # 20-day SMA

    # Define trading signals
    original_data["Signal"] = 0  # Initialize the 'Signal' column with zeros
    # Generating Buy and Sell signals based on crossover conditions
    original_data.loc[original_data["SMA_9"] > original_data["SMA_21"], "Signal"] = (
        1  # Buy signal
    )
    original_data.loc[
        original_data["SMA_9"] < original_data["SMA_21"], "Signal"
    ] = -1  # Sell signal

    data = data.with_columns(
        (data["('Close', 'AAPL')"].rolling_mean(9)).alias("SMA_9"),
        (data["('Close', 'AAPL')"].rolling_mean(21)).alias("SMA_21"),
    )

    data = data.with_columns(
        pl.when(pl.col("SMA_9") > pl.col("SMA_21"))
        .then(1)
        .otherwise(-1)
        .alias("Signal"),
    )

    print(original_data.index)
    print(data.tail(10))
    print(original_data.tail(10))
    # visulize_strategy(original_data)
    visulize_strategy(data)


if __name__ == "__main__":
    main()
