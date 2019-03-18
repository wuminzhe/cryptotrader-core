use crate::error::*;
use crate::models::*;
use crate::utils::*;

#[derive(Debug, Clone)]
pub struct Position {
    pub trades: Vec<Trade>,
    pub asset: Asset, // FIXME: could cause bugs if there are multiple asset types.
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PositionState {
    Open,
    Partial,
    Closed,
    Irreconciled, // oversold vs assets
    Invalid,      // when things don't make sense
}

impl ::std::fmt::Display for PositionState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            PositionState::Open => write!(f, "OPEN"),
            PositionState::Partial => write!(f, "PART"),
            PositionState::Closed => write!(f, "CLOSED"),
            PositionState::Invalid => write!(f, "INVALID"),
            PositionState::Irreconciled => write!(f, "IRREC"),
        }
    }
}

fn get_trades_for_qty(trades: &Vec<Trade>, qty: f64) -> Vec<Trade> {
    let trades: Vec<Trade> = trades.iter().cloned().rev().collect();
    let mut position_trades: Vec<Trade> = Vec::new();
    let mut remaining_qty = qty;
    // let trade_type = trades.first().expect("need a trade type").

    for trade in trades.clone() {
        if remaining_qty.round() <= 0.0 {
            break;
        }

        match trade.trade_type {
            TradeType::Buy => {
                remaining_qty = remaining_qty - trade.qty;
            }
            TradeType::Sell => {
                remaining_qty = remaining_qty + trade.qty;
            }
        };

        position_trades.push(trade.clone());
    }

    position_trades
}

impl Position {
    pub fn new(trades: Vec<Trade>, asset: Asset) -> CoreResult<Self> {
        // let mut asset_qty = asset.amount;
        // let mut position_trades: Vec<Trade> = Vec::new();

        // // let grouped_trades = group_trades_by_trade_type_pair()

        // for trade in trades {
        //     if asset_qty < 0. {
        //         break;
        //     }

        //     match trade.trade_type {
        //         TradeType::Buy => {
        //             asset_qty = asset_qty - trade.qty;
        //         }
        //         TradeType::Sell => {
        //             asset_qty = asset_qty + trade.qty;
        //         }
        //     };

        //     println!(
        //         "asset_qty is now: {} for {} (wallet qty: {})",
        //         asset_qty.clone(),
        //         trade.pair.clone(),
        //         asset.amount,
        //     );

        //     position_trades.push(trade.clone());

        //     // asset_qty = asset_qty - trade.qty;
        // }

        // while asset_qty > 0. {
        //     if let Some(trade) = trades.first() {
        //         asset_qty = asset_qty - trade.qty;

        //         println!(
        //             "asset_qty is now: {} for {} (wallet qty: {})",
        //             asset_qty.clone(),
        //             trade.pair.clone(),
        //             asset.amount,
        //         );

        //         position_trades.push(trade.clone());
        //     }
        // }

        if trades.is_empty() {
            return Err(Box::new(TrailerError::Generic(format!(
                "cannot create a position for {} without trades.",
                asset
            ))));
        };

        Ok(Position {
            trades: get_trades_for_qty(&trades, asset.amount),
            asset,
        })
    }

    pub fn symbol(&self) -> String {
        self.trades
            .first()
            .map(|trade| trade.pair.symbol.clone())
            .unwrap_or("ERROR".to_string())
    }

    pub fn entry_price(&self) -> f64 {
        let entry_prices: f64 = self.buy_trades().into_iter().map(|o| o.price * o.qty).sum();
        let total_qty: f64 = self.buy_trades().into_iter().map(|o| o.qty).sum();

        entry_prices / total_qty
    }

    pub fn exit_price(&self) -> Option<f64> {
        if self.sell_trades().len() > 0 {
            Some(
                self.sell_trades().into_iter().map(|t| t.price).sum::<f64>()
                    / self.sell_trades().len() as f64,
            )
        } else {
            None
        }
    }

    pub fn current_price(&self) -> f64 {
        self.buy_trades()
            .into_iter()
            .map(|o| o.pair.price)
            .sum::<f64>()
            / self.buy_trades().len() as f64
    }

    pub fn qty(&self) -> f64 {
        self.buy_qty() - self.sell_qty()
    }

    pub fn buy_qty(&self) -> f64 {
        self.buy_trades().into_iter().map(|o| o.qty).sum()
    }
    pub fn sell_qty(&self) -> f64 {
        self.sell_trades().into_iter().map(|o| o.qty).sum()
    }

    pub fn buy_cost(&self) -> f64 {
        self.entry_price() * self.buy_qty()
    }

    pub fn sell_cost(&self) -> f64 {
        self.exit_price().unwrap_or(0.0) * self.sell_qty()
    }

    // todo: memoize
    // pub fn compact_orders(&self) -> Vec<Order> {
    //  Order::group_by_price(self.orders.clone())
    // }

    // todo: memoize
    pub fn buy_trades(&self) -> Vec<Trade> {
        self.trades
            .clone()
            .into_iter()
            .filter(|t| t.trade_type == TradeType::Buy)
            .collect()
    }

    // todo: memoize
    pub fn sell_trades(&self) -> Vec<Trade> {
        self.trades
            .clone()
            .into_iter()
            .filter(|t| t.trade_type == TradeType::Sell)
            .collect()
    }

    /// averaged buy trade
    pub fn buy_trade(&self) -> Trade {
        average_trades(self.buy_trades())
    }

    /// averaged sell trade
    pub fn sell_trade(&self) -> Trade {
        average_trades(self.sell_trades())
    }

    pub fn remaining_qty(&self) -> f64 {
        // println!("remaining_qty: {}", self.asset.amount);
        self.asset.amount
        // self.buy_qty() - self.sell_qty()
    }

    pub fn state(&self) -> PositionState {
        derive_state(self.buy_qty(), self.sell_qty())
    }

    pub fn current_profit_as_percent(&self) -> f64 {
        // log::info!("{} {}, {}", self.trade_type, self.entry_price(), self.current_price());
        price_percent(self.entry_price(), self.current_price())
    }

    pub fn base_type(&self) -> Option<AssetType> {
        self.trades.first().map(|t| t.pair.base_type())
    }
}

/// group orders into buy-sell buy-sell buy-sell
// pub fn group_orders_by_positions(orders: Vec<Order>) -> Vec<(Vec<Order>)> {
//     let mut positions = Vec::new();
//     let mut current_orders: Vec<Order> = Vec::new();
//     let mut orders: Vec<Order> = orders.into_iter().rev().collect();

//     while let Some(last_order) = orders.pop() {
//         match last_order.order_type {
//             TradeType::Buy => {
//                 // if the list contains sells, and we've encountered a buy, lets toss it
//                 if current_orders
//                     .clone()
//                     .into_iter()
//                     .filter(|o| o.order_type == TradeType::Sell)
//                     .collect::<Vec<Order>>()
//                     .len()
//                     > 0
//                 {
//                     positions.push(current_orders.clone());
//                     current_orders = Vec::new();
//                 }
//             }
//             TradeType::Sell => {}
//         }
//         current_orders.push(last_order.clone());
//     }

//     positions.push(current_orders.clone());
//     positions
// }

pub fn derive_state(buy_qty: f64, sell_qty: f64) -> PositionState {
    if sell_qty == 0.0 {
        return PositionState::Open;
    };
    if buy_qty == sell_qty {
        return PositionState::Closed;
    };
    if sell_qty < buy_qty {
        return PositionState::Partial;
    };
    PositionState::Irreconciled
}

// mod tests {
//     use super::*;

//     fn order_fixture(order_type: TradeType, qty: f64, price: f64) -> Order {
//         Order {
//             id: "".to_string(),
//             symbol: "".to_string(),
//             order_type,
//             qty,
//             price,
//         }
//     }

//     #[test]
//     fn test_group_orders_by_positions_1() {
//         let orders = group_trades_by_positions(vec![order_fixture(TradeType::Buy, 10.0, 100.0)]);

//         assert_eq!(orders.len(), 1);
//         assert_eq!(orders.first().unwrap().len(), 1);
//     }

//     #[test]
//     fn test_group_orders_by_positions_2() {
//         let orders = group_trades_by_positions(vec![
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//             order_fixture(TradeType::Buy, 2.0, 100.0),
//         ]);

//         assert_eq!(orders.len(), 1);
//         assert_eq!(orders.first().unwrap().len(), 2);
//     }

//     #[test]
//     fn test_group_orders_by_positions_3() {
//         let orders = group_orders_by_positions(vec![
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//             order_fixture(TradeType::Buy, 2.0, 100.0),
//             order_fixture(TradeType::Sell, 3.0, 100.0),
//         ]);

//         assert_eq!(orders.len(), 1);
//         assert_eq!(orders.first().unwrap().len(), 3);
//     }

//     #[test]
//     fn test_group_orders_by_positions_4() {
//         let orders = group_orders_by_positions(vec![
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//             order_fixture(TradeType::Sell, 2.0, 100.0),
//             order_fixture(TradeType::Buy, 3.0, 100.0),
//         ]);

//         assert_eq!(orders.len(), 2);
//         assert_eq!(orders.first().unwrap().len(), 2);
//         assert_eq!(orders.last().unwrap().len(), 1);
//     }

//     #[test]
//     fn test_group_orders_by_positions_5() {
//         let orders = group_orders_by_positions(vec![
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//             order_fixture(TradeType::Sell, 2.0, 100.0),
//             order_fixture(TradeType::Buy, 3.0, 100.0),
//             order_fixture(TradeType::Sell, 4.0, 100.0),
//             order_fixture(TradeType::Buy, 5.0, 100.0),
//         ]);

//         assert_eq!(orders.len(), 3);

//         let first_order = orders.first().unwrap();
//         let last_order = orders.last().unwrap();

//         assert_eq!(first_order.len(), 2);
//         assert_eq!(last_order.len(), 1);
//     }

//     #[test]
//     fn test_group_orders_by_positions_6() {
//         let orders = group_orders_by_positions(vec![
//             order_fixture(TradeType::Buy, 2.0, 100.0),
//             order_fixture(TradeType::Buy, 5.0, 100.0),
//             order_fixture(TradeType::Buy, 3.0, 100.0),
//             order_fixture(TradeType::Sell, 1.0, 100.0),
//             order_fixture(TradeType::Sell, 1.0, 100.0),
//             order_fixture(TradeType::Sell, 8.0, 100.0),
//             order_fixture(TradeType::Buy, 3.0, 100.0),
//             order_fixture(TradeType::Sell, 4.0, 100.0),
//         ]);

//         assert_eq!(orders.len(), 2);

//         let first_order_group = orders.first().unwrap();
//         let second_order_group = orders.last().unwrap();

//         assert_eq!(first_order_group.len(), 6);
//         assert_eq!(second_order_group.len(), 2);
//     }

//     #[test]
//     fn test_positions_1() {
//         let positions = Position::new(vec![order_fixture(TradeType::Buy, 10.0, 100.0)]);

//         assert_eq!(positions.len(), 1);

//         let first_position = positions.first().unwrap();
//         println!("{:?}", first_position.buy_orders());
//         assert_eq!(first_position.orders.len(), 1);
//         assert_eq!(first_position.buy_orders().len(), 1);
//         assert_eq!(first_position.buy_qty(), 10.0);
//         assert_eq!(first_position.entry_price(), 100.0);
//         assert_eq!(first_position.exit_price(), None);
//         assert_eq!(first_position.buy_qty(), 10.0);
//     }

//     #[test]
//     fn test_positions_2() {
//         let positions = Position::new(vec![
//             order_fixture(TradeType::Buy, 10.0, 100.0),
//             order_fixture(TradeType::Buy, 10.0, 200.0),
//         ]);

//         assert_eq!(positions.len(), 1);

//         let first_position = positions.first().unwrap();
//         assert_eq!(first_position.orders.len(), 2);
//         assert_eq!(first_position.buy_orders().len(), 2);
//         assert_eq!(first_position.sell_orders().len(), 0);
//         assert_eq!(first_position.buy_qty(), 20.0);
//         assert_eq!(first_position.entry_price(), 150.0);
//         assert_eq!(first_position.exit_price(), None);
//         assert_eq!(first_position.buy_qty(), 20.0);
//         assert_eq!(first_position.sell_qty(), 0.0);
//     }

//     #[test]
//     fn test_positions_3() {
//         let positions = Position::new(vec![
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//             order_fixture(TradeType::Sell, 2.0, 100.0),
//             order_fixture(TradeType::Buy, 3.0, 100.0),
//             order_fixture(TradeType::Sell, 4.0, 100.0),
//             order_fixture(TradeType::Buy, 5.0, 100.0),
//             order_fixture(TradeType::Buy, 6.0, 200.0),
//         ]);

//         assert_eq!(positions.len(), 3);

//         let first_position = positions.first().unwrap();
//         assert_eq!(first_position.buy_orders().len(), 1);
//         assert_eq!(first_position.buy_qty(), 1.0);

//         let last_position = positions.last().unwrap();
//         assert_eq!(last_position.orders.len(), 2);
//         assert_eq!(last_position.buy_orders().len(), 2);
//         assert_eq!(last_position.buy_qty(), 11.0);
//     }

//     #[test]
//     fn test_positions_state_open() {
//         let positions = Position::new(vec![
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//         ]);
//         let position = positions.first().unwrap();

//         assert_eq!(position.state(), PositionState::Open);
//     }

//     #[test]
//     fn test_positions_state_closed() {
//         let positions = Position::new(vec![
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//             order_fixture(TradeType::Sell, 1.0, 100.0),
//         ]);
//         let position = positions.first().unwrap();

//         assert_eq!(position.state(), PositionState::Closed);
//     }

//     #[test]
//     fn test_positions_state_irec() {
//         let positions = Position::new(vec![
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//             order_fixture(TradeType::Sell, 2.0, 100.0),
//         ]);
//         let position = positions.first().unwrap();

//         assert_eq!(position.state(), PositionState::Irreconciled);
//     }

//     #[test]
//     fn test_positions_state_partial_1() {
//         let positions = Position::new(vec![
//             order_fixture(TradeType::Buy, 2.0, 100.0),
//             order_fixture(TradeType::Sell, 1.0, 100.0),
//         ]);
//         let position = positions.first().unwrap();

//         assert_eq!(position.state(), PositionState::Partial);
//         assert_eq!(position.remaining_qty(), 1.0);
//     }

//     #[test]
//     fn test_positions_state_partial_2() {
//         let positions = Position::new(vec![
//             order_fixture(TradeType::Buy, 2.0, 100.0),
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//             order_fixture(TradeType::Buy, 5.0, 200.0),
//             order_fixture(TradeType::Sell, 1.0, 100.0),
//             order_fixture(TradeType::Sell, 2.0, 100.0),
//         ]);

//         let position = positions.first().unwrap();

//         assert_eq!(position.state(), PositionState::Partial);
//         assert_eq!(position.remaining_qty(), 5.0);
//         assert_eq!(position.buy_orders().len(), 3);
//         assert_eq!(position.sell_orders().len(), 2);
//     }

//     #[test]
//     fn test_positions_state_closed_1() {
//         let positions = Position::new(vec![
//             order_fixture(TradeType::Buy, 2.0, 100.0),
//             order_fixture(TradeType::Buy, 1.0, 100.0),
//             order_fixture(TradeType::Buy, 5.0, 200.0),
//             order_fixture(TradeType::Sell, 1.0, 300.0),
//             order_fixture(TradeType::Sell, 7.0, 300.0),
//         ]);

//         let position = positions.first().unwrap();

//         assert_eq!(position.state(), PositionState::Closed);
//         assert_eq!(position.remaining_qty(), 0.0);
//         assert_eq!(position.buy_orders().len(), 3);
//         assert_eq!(position.sell_orders().len(), 2);
//     }
// }#[cfg(test)]
