# 通用金融交易 YAML DSL 規範

# 目錄

1.  [頂層結構](#1-頂層結構)
2.  [通用資產定義](#2-通用資產定義)
3.  [通用指標系統](#3-通用指標系統)
4.  [條件系統](#4-條件系統)
5.  [動作系統](#5-動作系統)
6.  [風險管理](#6-風險管理)
7.  [執行控制](#7-執行控制)
8.  [狀態機定義](#8-狀態機定義)
9.  [完整策略範例](#9-完整策略範例)
10. [擴展機制](#10-擴展機制)
11. [基本迴圈結構](#11-基本迴圈結構)
12. [巢狀迴圈](#12-巢狀迴圈)
13. [條件迴圈 (WHILE)](#13-條件迴圈-while)
14. [迴圈中的集合操作](#14-迴圈中的集合操作)
15. [時間序列迴圈](#15-時間序列迴圈)
16. [動態迴圈產生](#16-動態迴圈產生)
17. [實際策略範例](#17-實際策略範例)
18. [迴圈中的狀態管理](#18-迴圈中的狀態管理)
19. [平行迴圈執行](#19-平行迴圈執行)
20. [錯誤處理和中斷](#20-錯誤處理和中斷)
21. [嵌套規則](#21-嵌套規則)
22. [基本嵌套範例](#22-基本嵌套範例)
23. [三層嵌套](#33-三層嵌套)
24. [混合迴圈類型](#24-混合迴圈類型)
25. [條件嵌套](#25-條件嵌套)
26. [嵌套中的數據共享](#26-嵌套中的數據共享)
27. [平行嵌套執行](#27-平行嵌套執行)
28. [遞迴模式（特殊情況）](#28-遞迴模式特殊情況)
29. [錯誤處理和效能優化](#29-錯誤處理和效能優化)
30. [實戰範例：複雜的配對交易策略](#30-實戰範例複雜的配對交易策略)

## 1. 頂層結構
# DSL 元數據
```
dsl_version: "2.0"
created_at: "2024-01-01T00:00:00Z"
author: "strategy_author"

# 策略定義
strategy:
  name: string
  version: string
  description: string
  tags: [string]
  
  # 資產配置
  universe:
    asset_classes: [stocks, futures, crypto, forex, options]
    markets: [TW, US, CN, GLOBAL]
    exchanges: [TWSE, TPEx, NYSE, NASDAQ, BINANCE, CME]
    
  # 執行環境
  execution:
    mode: [backtest, paper, live]
    frequency: [tick, second, minute, hour, daily]
    timezone: string
```
## 2. 通用資產定義
# 資產類型定義
```
assets:
  stocks:
    symbols: ["2330.TW", "AAPL.US"]
    filters:
      market_cap: {min: 1000000, max: null}
      price: {min: 10, max: 1000}
      volume: {min: 1000000}
      
  futures:
    contracts: ["TWTX", "ES", "CL"]
    month_codes: ["current", "next", "quarter"]
    filters:
      open_interest: {min: 1000}
      
  crypto:
    pairs: ["BTC/USDT", "ETH/USDT"]
    exchanges: ["BINANCE", "COINBASE"]
    
  forex:
    pairs: ["EUR/USD", "USD/JPY"]
    
  options:
    underlying: ["AAPL", "TSLA"]
    types: ["call", "put"]
    expiry: ["weekly", "monthly"]
```
## 3. 通用指標系統
# 可擴展的指標定義
```
indicators:
  # 技術分析指標
  technical:
    - name: ma
      params: {period: int, type: [simple, exponential, weighted]}
    - name: rsi
      params: {period: int}
    - name: macd
      params: {fast: int, slow: int, signal: int}
    - name: bollinger_bands
      params: {period: int, std_dev: float}
    - name: atr
      params: {period: int}
    - name: stochastic
      params: {k_period: int, d_period: int}
    
  # 基本面指標
  fundamental:
    - name: pe_ratio
    - name: price_to_book
    - name: revenue_growth
    - name: debt_to_equity
    
  # 市場微結構
  microstructure:
    - name: bid_ask_spread
    - name: order_book_imbalance
    - name: volume_profile
    - name: time_and_sales
    
  # 衍生品特有
  derivatives:
    - name: implied_volatility
    - name: option_greeks
    - name: futures_basis
    - name: open_interest
    
  # 情緒指標
  sentiment:
    - name: put_call_ratio
    - name: vix
    - name: social_sentiment
    - name: news_sentiment
    
  # 自定義指標
  custom:
    - name: my_indicator
      formula: "(close - open) / open * 100"
      inputs: [close, open]
      output: percentage
```
## 4. 條件系統
# 條件定義語法
```
conditions:
  # 基本比較
  comparisons:
    - price > ma(20)
    - volume >= avg_volume * 1.5
    - rsi(14) < 30
    
  # 複雜邏輯
  logical:
    all_of:  # AND
      - price > ma(50)
      - rsi(14) < 70
    any_of:  # OR
      - volume > prev_volume * 2
      - price_change > 3%
    none_of:  # NOT
      - in_downtrend
      - holiday
      
  # 時間條件
  temporal:
    time_of_day:
      start: "09:30"
      end: "15:00"
    days_of_week: [MON, TUE, WED, THU, FRI]
    exclude_dates: ["2024-01-01", "2024-02-10"]
    
  # 跨資產條件
  cross_asset:
    - symbol("TWTX").price > symbol("TWTX").ma(20)
    - correlation("BTC/USDT", "ETH/USDT", 20) > 0.7
    
  # 市場狀態
  market_state:
    - market_trend("US") == "BULL"
    - vix < 20
    - sector("TECH").performance > market.performance
```
## 5. 動作系統
# 交易動作定義
```
actions:
  # 基本動作
  basic:
    - type: buy
      quantity: 100
      order_type: [market, limit, stop]
      
    - type: sell
      quantity: all
      
    - type: short
      quantity: {calculate: "capital * 0.1 / price"}
      
  # 進階訂單
  advanced:
    - type: bracket_order
      entry: {type: limit, price: current_price * 0.99}
      stop_loss: {type: stop, price: entry_price * 0.97}
      take_profit: {type: limit, price: entry_price * 1.03}
      
    - type: iceberg_order
      total_quantity: 10000
      visible_quantity: 100
      
  # 期貨特有
  futures_specific:
    - type: roll_contract
      from: current_month
      to: next_month
      when: days_to_expiry < 5
      
  # 選擇權特有
  options_specific:
    - type: buy_call
      strike: {at_the_money: true}
      expiry: next_friday
      
    - type: sell_put
      strike: {delta: -0.3}
      
    - type: iron_condor
      call_spread: {strikes: [110, 115]}
      put_spread: {strikes: [90, 85]}
      
  # 加密貨幣特有
  crypto_specific:
    - type: stake
      token: "ETH"
      amount: 32
      
    - type: provide_liquidity
      pair: "BTC/USDT"
      amount: {token1: 1, token2: 30000}
```
## 6. 風險管理
```
risk_management:
  # 部位層級
  position_level:
    stop_loss:
      type: [fixed, trailing, atr_based]
      value: 3%
    take_profit:
      type: percentage
      value: 10%
    max_holding_period: 30_days
    
  # 策略層級
  strategy_level:
    max_positions: 10
    max_exposure: 0.3  # 30% of capital
    max_sector_exposure: 0.15
    max_correlation: 0.7
    
  # 投資組合層級
  portfolio_level:
    max_drawdown: 0.2
    var_limit: 0.05  # 5% VaR
    sharpe_ratio_target: 1.5
    
  # 動態調整
  dynamic_sizing:
    volatility_adjusted: true
    kelly_criterion: 0.25  # 25% Kelly
    risk_parity: true
```
## 7. 執行控制
```
execution:
  # 滑價模型
  slippage:
    model: [fixed, linear, square_root]
    parameter: 0.1%
    
  # 手續費
  commission:
    stocks: {rate: 0.001, minimum: 1}
    futures: {per_contract: 2.5}
    crypto: {maker: 0.0002, taker: 0.0004}
    
  # 執行限制
  constraints:
    min_order_size: 100
    max_order_size: 10000
    max_orders_per_minute: 10
    trading_hours:
      regular: "09:30-16:00"
      extended: "07:00-20:00"
```
## 8. 狀態機定義
# 策略狀態管理
```
state_machine:
  initial: searching
  
  states:
    searching:
      description: "尋找進場機會"
      
    position_opened:
      description: "已建立部位"
      sub_states:
        - scaling_in
        - holding
        - scaling_out
        
    position_closed:
      description: "部位已關閉"
      
  transitions:
    - from: searching
      to: position_opened
      trigger: entry_signal
      action: open_position
      
    - from: position_opened
      to: position_closed
      trigger: [stop_loss_hit, take_profit_hit, exit_signal]
      action: close_position
```
## 9. 完整策略範例
# 跨市場動量策略
```
strategy:
  name: "Multi-Asset Momentum"
  version: "1.0"
  
  universe:
    asset_classes: [stocks, futures, crypto]
    markets: [US, TW, GLOBAL]
    
  parameters:
    lookback_period: 20
    momentum_threshold: 0.02
    position_size: 0.1
    
  indicators:
    - name: momentum
      formula: "(price - price[lookback_period]) / price[lookback_period]"
      
  screens:
    pre_market:
      all_of:
        - volume > ma(volume, 20)
        - price > 10
        - market_cap > 1_000_000_000
        
  signals:
    entry:
      name: momentum_breakout
      when:
        all_of:
          - momentum > momentum_threshold
          - price > ma(price, 50)
          - rsi(14) < 70
      actions:
        - type: buy
          quantity: {calculate: "capital * position_size / price"}
          
    exit:
      name: momentum_reversal
      when:
        any_of:
          - momentum < 0
          - price < ma(price, 20)
          - holding_period > 30
      actions:
        - type: sell
          quantity: all
          
  risk_management:
    position_level:
      stop_loss: {type: trailing, value: 5%}
      take_profit: {type: fixed, value: 20%}
    portfolio_level:
      max_positions: 20
      max_sector_exposure: 0.25
```
## 10. 擴展機制
# 插件系統
```
plugins:
  # 數據源插件
  data_sources:
    - name: yahoo_finance
      type: historical
      assets: [stocks, etf]
      
    - name: binance_api
      type: real_time
      assets: [crypto]
      
  # 分析插件
  analytics:
    - name: ml_predictor
      model: random_forest
      features: [rsi, macd, volume_profile]
      
    - name: sentiment_analyzer
      sources: [twitter, reddit, news]
      
  # 執行插件
  execution:
    - name: smart_router
      venues: [exchange, dark_pool, otc]
      algorithm: best_execution
      
# 自定義擴展
extensions:
  custom_indicators:
    - name: my_complex_indicator
      language: python
      code: |
        def calculate(data):
            return data['close'].rolling(20).std() / data['close'].mean()
            
  custom_conditions:
    - name: market_regime
      code: |
        def check(data):
            return data['vix'] < 20 and data['spy_trend'] == 'up'
```
## 11. 基本迴圈結構
# FOR 迴圈
```
loops:
  - type: for
    name: scan_multiple_emas
    iterator: period
    values: [10, 20, 50, 100, 200]
    condition:
      - price > ema(period)
    action:
      type: add_to_watchlist
      tag: "above_ema_{period}"

  # FOR 範圍迴圈
  - type: for_range
    name: test_multiple_stops
    variable: stop_percentage
    from: 1
    to: 5
    step: 0.5
    action:
      type: set_stop_loss
      value: "{stop_percentage}%"

  # FOR EACH 迴圈（遍歷集合）
  - type: foreach
    name: check_all_positions
    collection: open_positions
    as: position
    when:
      - position.profit_percent > 10
    action:
      type: partial_close
      quantity: position.quantity * 0.5
```
## 12. 巢狀迴圈
# 雙重迴圈範例
```
nested_loops:
  - type: for
    name: scan_ma_crosses
    iterator: fast_period
    values: [5, 10, 20]
    inner_loop:
      type: for
      iterator: slow_period
      values: [20, 50, 100]
      where: slow_period > fast_period  # 避免無效組合
      condition:
        - ma(fast_period) > ma(slow_period)
        - ma(fast_period)[-1] <= ma(slow_period)[-1]  # 黃金交叉
      action:
        type: signal
        name: "ma_cross_{fast_period}_{slow_period}"
```
## 13. 條件迴圈 (WHILE)
# WHILE 迴圈
```
while_loops:
  - type: while
    name: pyramid_buying
    condition: position_size < max_position_size
    max_iterations: 5  # 安全限制
    body:
      when:
        - price < last_buy_price * 0.98
        - available_capital > min_order_size
      action:
        type: buy
        quantity: base_unit
      update:
        - position_size += base_unit
        - last_buy_price = current_price
```
## 14. 迴圈中的集合操作
# 集合操作範例
```
collection_operations:
  - type: map
    name: calculate_all_rsi
    collection: watchlist_symbols
    operation:
      calculate: rsi(14)
      store_as: "{symbol}_rsi"

  - type: filter
    name: filter_strong_stocks
    collection: all_stocks
    condition:
      - price > ma(50)
      - volume > avg_volume * 1.5
    result: strong_stocks

  - type: reduce
    name: total_exposure
    collection: open_positions
    operation: sum
    field: position.value
    result: total_portfolio_value
```
## 15. 時間序列迴圈
# 時間序列迴圈
```
time_series_loops:
  - type: for_timeframe
    name: intraday_scan
    from: "09:30"
    to: "16:00"
    interval: 5_minutes
    action:
      check_condition:
        - vwap > price
      execute:
        type: short
        quantity: 100

  - type: for_dates
    name: monthly_rebalance
    dates:
      - every: month_end
      - specific: ["2024-01-31", "2024-02-29"]
    action:
      type: rebalance_portfolio
      target_weights:
        stocks: 0.6
        bonds: 0.4
```
## 16. 動態迴圈產生
# 動態產生迴圈
```
dynamic_loops:
  - type: generate_loop
    name: adaptive_stop_loss
    based_on: volatility
    formula: |
      periods = [10, 20, 50] if volatility > 0.02 else [20, 50, 100]
      multipliers = linspace(1.5, 3.0, steps=5)
    
  - type: conditional_loop
    name: sector_rotation
    if: market_regime == "risk_off"
    then:
      foreach: defensive_sectors
      action: increase_allocation
    else:
      foreach: growth_sectors  
      action: increase_allocation
```
## 17. 實際策略範例
```
strategy:
  name: "Multi-Timeframe Scanner"
  
  # 掃描多個時間框架
  signals:
    - type: for
      name: timeframe_alignment
      iterator: timeframe
      values: ["5m", "15m", "1h", "4h", "1d"]
      condition_template:
        all_of:
          - rsi(14, timeframe) < 70
          - price > ma(20, timeframe)
      store_results: timeframe_signals
      
    # 當多個時間框架對齊時進場
    - name: entry_signal
      when:
        - count(timeframe_signals.positive) >= 3
      action:
        type: buy
        quantity: 1000

  # 使用迴圈進行資金管理
  position_sizing:
    - type: foreach
      name: risk_based_sizing
      collection: potential_trades
      as: trade
      calculate:
        size: |
          volatility = atr(14, trade.symbol)
          risk_amount = account_balance * 0.01
          position_size = risk_amount / (volatility * 2)
      action:
        type: set_position_size
        symbol: trade.symbol
        quantity: position_size

  # 動態停損調整
  risk_management:
    - type: for_range
      name: trailing_stop_ladder
      variable: profit_level
      from: 2
      to: 10
      step: 2
      when:
        - position.profit_percent >= profit_level
      action:
        type: adjust_stop_loss
        to: entry_price * (1 + profit_level * 0.005)
```
## 18. 迴圈中的狀態管理
# 狀態管理迴圈
```
stateful_loops:
  - type: for
    name: accumulation_phase
    iterator: day
    values: range(1, 30)
    maintain_state:
      total_bought: 0
      average_price: 0
    
    condition:
      - price < ma(50)
      - total_bought < max_position
    
    action:
      type: buy
      quantity: daily_amount
      
    update_state:
      total_bought: total_bought + daily_amount
      average_price: |
        (average_price * total_bought + price * daily_amount) / 
        (total_bought + daily_amount)
```
## 19. 平行迴圈執行
# 平行執行迴圈
```
parallel_execution:
  - type: parallel_for
    name: multi_symbol_scan
    collection: universe.symbols
    max_workers: 10
    task:
      calculate_indicators:
        - rsi(14)
        - macd(12, 26, 9)
        - bb_bands(20, 2)
      check_conditions:
        - price > bb_upper
        - rsi > 70
      action_if_true:
        type: add_alert
        message: "Overbought condition"
```
## 20. 錯誤處理和中斷
# 迴圈控制
```
loop_control:
  - type: for
    name: retry_order
    iterator: attempt
    values: range(1, 5)
    
    action:
      type: place_order
      
    on_error:
      - log: "Order failed, attempt {attempt}"
      - wait: 2_seconds
      - continue  # 繼續下一次迭代
      
    on_success:
      - break  # 成功後跳出迴圈
      
    finally:
      - log: "Order process completed"
```
## 21. 嵌套規則
```
nesting_rules:
  # 支援的嵌套組合
  allowed_nesting:
    - for_loop:
        can_contain: [for, foreach, while, map, filter]
    - foreach_loop:
        can_contain: [for, foreach, while, map, filter]
    - while_loop:
        can_contain: [for, foreach, while, map, filter]
    - parallel_for:
        can_contain: [for, foreach, map, filter]  # 不建議嵌套 while
        
  # 嵌套深度限制（可配置）
  max_nesting_depth: 5
  
  # 效能保護
  performance_guards:
    max_total_iterations: 10000
    timeout_seconds: 300
```
## 22. 基本嵌套範例
# FOR 中嵌套 FOREACH
```
nested_example_1:
  - type: for
    name: scan_multiple_timeframes
    iterator: timeframe
    values: ["5m", "15m", "1h", "4h"]
    body:
      - type: foreach
        name: check_symbols
        collection: watchlist_symbols
        as: symbol
        condition:
          - symbol.rsi(14, timeframe) < 30
          - symbol.volume > ma(symbol.volume, 20, timeframe)
        action:
          type: add_signal
          details:
            symbol: "{symbol}"
            timeframe: "{timeframe}"
            signal: "oversold"
```
## 33. 三層嵌套
# FOR -> FOREACH -> WHILE
```
complex_nesting:
  - type: for
    name: portfolio_optimization
    iterator: risk_level
    values: [0.01, 0.02, 0.03]  # 1%, 2%, 3% risk
    body:
      - type: foreach
        name: sector_allocation
        collection: market_sectors
        as: sector
        body:
          - type: while
            name: position_building
            condition: 
              - sector.allocation < target_allocation
              - available_capital > min_trade_size
            max_iterations: 10
            body:
              find_best_stock:
                in: sector.stocks
                criteria:
                  - pe_ratio < sector.average_pe
                  - rsi(14) < 50
              action:
                type: buy
                symbol: best_stock
                quantity: calculate_size(risk_level)
              update:
                sector.allocation += position_value
```
## 24. 混合迴圈類型
# 組合不同迴圈類型
```
mixed_loops:
  - type: foreach
    name: multi_asset_scan
    collection: asset_classes
    as: asset_class
    body:
      - type: for_range
        name: volatility_scan
        variable: vol_threshold
        from: 0.1
        to: 0.5
        step: 0.1
        body:
          - type: filter
            name: filter_by_volatility
            collection: asset_class.instruments
            condition:
              - volatility(20) > vol_threshold
              - volatility(20) < vol_threshold + 0.1
            store_as: "vol_band_{vol_threshold}"
            
          - type: map
            name: calculate_positions
            collection: "vol_band_{vol_threshold}"
            operation:
              calculate_size:
                formula: "capital * 0.02 / (volatility * sqrt(252))"
```
## 25. 條件嵌套
# 根據條件決定嵌套結構
```
conditional_nesting:
  - type: for
    name: market_regime_strategy
    iterator: market
    values: ["US", "EU", "ASIA"]
    body:
      - type: switch
        on: market_condition(market)
        cases:
          bullish:
            - type: foreach
              collection: growth_stocks
              action: aggressive_buying
          bearish:
            - type: while
              condition: cash_ratio < 0.8
              action: reduce_positions
          sideways:
            - type: for_range
              variable: strike_distance
              from: -5
              to: 5
              step: 1
              action: sell_options
```
## 26. 嵌套中的數據共享
# 跨層級數據存取
```
data_sharing_example:
  - type: for
    name: outer_loop
    iterator: strategy_type
    values: ["momentum", "mean_reversion", "trend_following"]
    context:  # 外層上下文
      total_allocation: 0
      strategy_results: {}
    
    body:
      - type: foreach
        name: middle_loop
        collection: available_symbols
        as: symbol
        inherit_context: true  # 繼承父層上下文
        local_context:  # 本層上下文
          symbol_allocation: 0
          
        body:
          - type: for_range
            name: inner_loop
            variable: lookback
            from: 10
            to: 50
            step: 10
            inherit_context: true
            
            action:
              calculate:
                performance: backtest(strategy_type, symbol, lookback)
              update_context:
                # 更新各層上下文
                - level: local
                  set: best_lookback = lookback if performance > best_performance
                - level: parent
                  set: symbol_allocation += suggested_allocation
                - level: root
                  set: total_allocation += suggested_allocation
```
## 27. 平行嵌套執行
# 平行處理嵌套結構
```
parallel_nesting:
  - type: parallel_for
    name: market_scan
    collection: global_markets
    max_workers: 5
    body:
      - type: foreach
        name: sector_scan
        collection: market.sectors
        body:
          - type: parallel_for  # 嵌套平行處理
            name: stock_analysis
            collection: sector.stocks
            max_workers: 10
            body:
              compute:
                - technical_score
                - fundamental_score
                - sentiment_score
              
          - type: aggregate
            name: sector_summary
            operation: weighted_average
            weights: [0.4, 0.4, 0.2]
```
## 28. 遞迴模式（特殊情況）
# 遞迴處理層級結構
```
recursive_pattern:
  - type: recursive_foreach
    name: sector_hierarchy
    collection: market_sectors
    as: sector
    max_depth: 3
    body:
      process_sector:
        - calculate: sector_metrics
        - if: sector.has_subsectors
          then:
            - type: recursive_foreach
              collection: sector.subsectors
              use_same_pattern: true
```
## 29. 錯誤處理和效能優化
# 嵌套中的錯誤處理
```
nested_error_handling:
  - type: for
    name: robust_scan
    iterator: timeframe
    values: ["1m", "5m", "15m"]
    error_handling:
      strategy: continue  # 錯誤時繼續下一個
      log_errors: true
    
    body:
      - type: foreach
        name: symbol_check
        collection: symbols
        timeout: 30_seconds
        error_handling:
          strategy: skip  # 跳過有問題的符號
          fallback:
            action: mark_as_invalid
            
        body:
          - type: while
            name: retry_data_fetch
            condition: not data_received
            max_iterations: 3
            error_handling:
              on_timeout: break
              on_error: wait_and_retry
```
## 30. 實戰範例：複雜的配對交易策略
```
strategy:
  name: "Advanced Pairs Trading"
  
  # 三層嵌套實現配對交易
  pairs_discovery:
    - type: foreach
      name: sector_scan
      collection: market_sectors
      as: sector
      body:
        - type: for
          name: correlation_periods
          iterator: period
          values: [20, 60, 120]
          body:
            - type: foreach
              name: find_pairs
              collection: sector.stocks
              as: stock1
              body:
                - type: foreach
                  name: compare_stocks
                  collection: sector.stocks
                  as: stock2
                  where: stock2.symbol > stock1.symbol  # 避免重複
                  
                  calculate:
                    correlation: corr(stock1, stock2, period)
                    cointegration: coint_test(stock1, stock2)
                    
                  condition:
                    - correlation > 0.8
                    - cointegration.pvalue < 0.05
                    
                  action:
                    type: add_pair
                    pair:
                      stocks: [stock1, stock2]
                      period: period
                      stats:
                        correlation: correlation
                        half_life: calculate_half_life(stock1, stock2)
                        
        - type: while  # 持續監控配對
          name: monitor_pairs
          condition: market_open
          body:
            - type: foreach
              name: check_signals
              collection: discovered_pairs
              as: pair
              
              calculate:
                spread: pair.stock1.price - pair.hedge_ratio * pair.stock2.price
                z_score: (spread - spread.mean) / spread.std
                
              signals:
                entry_long:
                  when: z_score < -2
                  action:
                    - buy: pair.stock1
                    - short: pair.stock2
                    
                entry_short:
                  when: z_score > 2
                  action:
                    - short: pair.stock1
                    - buy: pair.stock2
                    
                exit:
                  when: abs(z_score) < 0.5
                  action: close_all_positions
```
## 31. 內建函數的具體定義
```
built_in_functions:
  # 數學函數
  math:
    - abs(x): "絕對值"
    - max(x, y, ...): "最大值"
    - min(x, y, ...): "最小值"
    - sum(array): "總和"
    - mean(array): "平均值"
    - std(array): "標準差"
    - sqrt(x): "平方根"
    - log(x): "自然對數"
    - exp(x): "指數函數"
    
  # 時間序列函數
  timeseries:
    - shift(series, n): "位移n個週期"
    - rolling(series, window): "滾動窗口"
    - resample(series, freq): "重新採樣"
    - first(series): "第一個值"
    - last(series): "最後一個值"
    
  # 統計函數
  statistics:
    - corr(x, y, period): "相關係數"
    - cov(x, y, period): "協方差"
    - percentile(array, p): "百分位數"
    - zscore(value, series): "Z分數"
```
## 32. 技術指標函數的參數規範
```
technical_indicators:
  ma:
    params:
      series: price_series
      period: int
      type: [simple, exponential, weighted]
    returns: numeric
    
  rsi:
    params:
      series: price_series  
      period: int
    returns: numeric (0-100)
    
  macd:
    params:
      series: price_series
      fast: int
      slow: int
      signal: int
    returns: 
      macd_line: numeric
      signal_line: numeric
      histogram: numeric
```
## 33. 數據存取函數
```
data_access:
  price_data:
    - open: "開盤價"
    - high: "最高價"
    - low: "最低價"
    - close: "收盤價"
    - volume: "成交量"
    - vwap: "成交量加權平均價"
    
  reference:
    - prev(series, n): "前n個值"
    - at(series, datetime): "特定時間的值"
    - between(series, start, end): "時間區間的值"
```
## 34. 類型系統和驗證規則
```
type_system:
  basic_types:
    - numeric: float/int
    - string: text
    - boolean: true/false
    - datetime: ISO8601
    - array: [type]
    - object: {key: type}
    
  validation_rules:
    - required_fields: [strategy.name, strategy.version]
    - type_checking: strict
    - range_validation: enabled
```
## 35. 標準常量定義
```
constants:
  time_frames:
    - tick
    - second
    - minute
    - hour
    - daily
    - weekly
    - monthly
    
  order_types:
    - market
    - limit
    - stop
    - stop_limit
    
  market_sessions:
    pre_market: "04:00-09:30"
    regular: "09:30-16:00"
    after_hours: "16:00-20:00"
```
