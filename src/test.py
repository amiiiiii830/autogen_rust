import yfinance as yf
import matplotlib.pyplot as plt

stock_data = yf.download('NVDA', period='1mo')
if stock_data.empty:
    print("Failed to retrieve data. Please try again later.")
else:
    plt.figure(figsize=(10, 5))
    plt.plot(stock_data.index, stock_data['Adj Close'].dropna())
    plt.title('Nvidia Stock Price Performance (Past Month)')
    plt.xlabel('Date')
    plt.ylabel('Stock Price (USD)')
    plt.show()