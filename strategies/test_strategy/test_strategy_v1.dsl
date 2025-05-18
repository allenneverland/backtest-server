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