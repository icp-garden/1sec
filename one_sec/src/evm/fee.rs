//! This module defines the transaction fee related types.
use std::collections::VecDeque;

use crate::numeric::{BlockNumber, GasAmount, Percent, Timestamp, Wei, WeiPerGas};

/// An EVM transaction fee estimate at a specific time.
#[derive(Debug, Default, Clone)]
pub struct FeeEstimate {
    pub fee: TxFee,
    pub block_number: BlockNumber,
    pub last_updated: Timestamp,
}

#[derive(Debug, Default, Clone)]
pub struct AverageFeeEstimate {
    sum: TxFee,
    count: usize,
}

impl AverageFeeEstimate {
    pub fn add(&mut self, fee: TxFee) {
        self.sum.max_fee_per_gas = self.sum.max_fee_per_gas.add(
            fee.max_fee_per_gas,
            "BUG: overflow in AverageFeeEstimate: max_fee_per_gas",
        );
        self.sum.max_priority_fee_per_gas = self.sum.max_priority_fee_per_gas.add(
            fee.max_priority_fee_per_gas,
            "BUG: overflow in AverageFeeEstimate: max_priority_fee_per_gas",
        );
        self.count += 1;
    }

    pub fn average(&self) -> Option<TxFee> {
        if self.count == 0 {
            None
        } else {
            Some(TxFee {
                max_fee_per_gas: WeiPerGas::new(
                    (self.sum.max_fee_per_gas.as_f64() / self.count as f64).round() as u128,
                ),
                max_priority_fee_per_gas: WeiPerGas::new(
                    (self.sum.max_priority_fee_per_gas.as_f64() / self.count as f64).round()
                        as u128,
                ),
            })
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct DailyFeeEstimate {
    hourly: VecDeque<(AverageFeeEstimate, Timestamp)>,
}

impl DailyFeeEstimate {
    pub fn add(&mut self, fee: &FeeEstimate) {
        const HOURS_PER_DAY: usize = 24;
        const MS_PER_HOUR: u64 = 60 * 60 * 1_000;
        fn hour(t: Timestamp) -> u64 {
            t.into_inner() / MS_PER_HOUR
        }

        let new_hour = match self.hourly.back() {
            Some((_avg, time)) => hour(*time) != hour(fee.last_updated),
            None => true,
        };

        if new_hour {
            self.hourly
                .push_back((AverageFeeEstimate::default(), fee.last_updated));
        }

        match self.hourly.back_mut() {
            Some((hourly, _time)) => {
                hourly.add(fee.fee.clone());
            }
            None => {
                unreachable!("BUG: impossible in DailyFeeEstimate")
            }
        }

        if self.hourly.len() > HOURS_PER_DAY {
            self.hourly.pop_front();
        }
    }

    pub fn average(&self) -> Option<TxFee> {
        let mut result = AverageFeeEstimate::default();
        for (hourly, _time) in self.hourly.iter() {
            if let Some(fee) = hourly.average() {
                result.add(fee);
            }
        }
        result.average()
    }
}

/// An EVM transaction fee.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TxFee {
    pub max_fee_per_gas: WeiPerGas,
    pub max_priority_fee_per_gas: WeiPerGas,
}

impl TxFee {
    /// Returns the larger of the two given fees.
    pub fn higher(&self, other: &TxFee) -> TxFee {
        TxFee {
            max_fee_per_gas: self.max_fee_per_gas.max(other.max_fee_per_gas),
            max_priority_fee_per_gas: self
                .max_priority_fee_per_gas
                .max(other.max_priority_fee_per_gas),
        }
    }

    /// Returns a fee increased by the given number of percent.
    pub fn bump(&self, percent: Percent) -> TxFee {
        fn do_bump(fee: WeiPerGas, percent: Percent) -> WeiPerGas {
            fee.checked_mul(percent.numerator())
                .and_then(|bump| bump.checked_div_floor(percent.denominator()))
                .and_then(|bump| fee.checked_add(bump))
                .unwrap_or_else(|| panic!("BUG: failed to bump fee {} by {}%", fee, percent))
        }
        TxFee {
            max_fee_per_gas: do_bump(self.max_fee_per_gas, percent),
            max_priority_fee_per_gas: do_bump(self.max_priority_fee_per_gas, percent),
        }
    }

    /// Returns the cost of executing a transaction with the given gas limit
    /// using the current fee.
    pub fn cost(&self, gas_limit: GasAmount, margin: Percent) -> Wei {
        self.bump(margin)
            .max_fee_per_gas
            .mul(
                gas_limit.into_inner(),
                "BUG: overflow in TransactionFee::cost",
            )
            .change_units()
    }
}
