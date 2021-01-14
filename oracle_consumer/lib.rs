#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod oracle_consumer {

    /// We add the type with currently supported Oracle results
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum OracleResult {
        Numeric(i64),
        RawBytes([u8; 32]),
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
    }

    #[ink(storage)]
    pub struct OracleConsumer {
        /// The smart contract of the Oracle we are inherently trusting
        /// with providing the data feeds
        authorized_oracle: AccountId,
        /// This is the value we will be updating trough the oracle
        /// It does not have to be the same size as OracleResult::Numeric
        bitcoin_price: u64,
    }

    impl OracleConsumer {

        #[ink(constructor)]
        pub fn new(authorized_oracle: AccountId, bitcoin_price: u64) -> Self {
            // set the oracle which will be allowed to update our bitcoin price
            // set the intial price on contract creation
            Self { authorized_oracle, bitcoin_price }
        }

        /// This method is called from the Oracle's `callback` fn.
        /// It can be named anything (in this case `set_bitcoin_price`),
        /// however it does need a fixed selector.
        /// The selector value needs to be the same as in the Oracle contract.
        #[ink(message, selector = "0xB16B00B5")]
        pub fn set_bitcoin_price(&mut self, result: OracleResult) -> Result<(),Error> {
            // check if the oracle is trusted
            let oracle = self.env().caller();
            if oracle != self.authorized_oracle {
                return Err(Error::Unauthorized);
            }

            // set the oracle's value
            if let OracleResult::Numeric(price) = result {
                self.bitcoin_price = price as u64;
            }

            // Let the oracle know all is good
            Ok(())
        }

        /// Meme function. Note that since smart contracts don't support
        /// floats, we deliberately encoded 8 decimal points of precision
        /// by using large ints.
        #[ink(message)]
        pub fn its_over_9000(&self) -> bool {
            self.bitcoin_price > 9000 as u64 * 1e8 as u64
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn default_works() {
            let oracle_stub: AccountId = [0x0; 32].into();
            let c = OracleConsumer::new(oracle_stub, 0);
            assert_eq!(c.its_over_9000(), false);
        }

    }
}
